#include "network.h"

#include <sstream>
#include <future>
#include <chrono>
#include <curlpp/cURLpp.hpp>
#include <curlpp/Easy.hpp>
#include <curlpp/Options.hpp>
#include <json/json.h>
#include <uWS/uWS.h>

std::string requestWebsocketURI(const std::string &url, bool verbose) {
	curlpp::Cleanup cleanup;
	curlpp::Easy req;

	req.setOpt(new curlpp::options::Url(url));
	req.setOpt(new curlpp::options::Post(true));

	std::stringstream stream;
	stream << req;

	Json::Value root;
	stream >> root;

	if (root["ok"].asBool()) {
		return root["url"].asString();
	} else {
		throw std::runtime_error(root["error"].isString() ? root["error"].asString() : "unknown error");
	}
}

void kristforge::network::run(const std::string &node, const std::shared_ptr<kristforge::State> &state, Options opts) {
	using namespace uWS;

	Hub hub;
	auto *const hubClient = dynamic_cast<Group<false> *>(&hub);

	std::mutex submitMtx;
	std::optional<long> submitWaitingID;
	std::condition_variable submitCV;

	hub.onConnection([&](WebSocket<false> *ws, const HttpRequest &req) {
		if (opts.onConnect) (*opts.onConnect)();
	});

	hub.onDisconnection([&](WebSocket<false> *ws, int code, char *msg, size_t length) {
		state->unsetTarget();
		if (opts.onDisconnect) (*opts.onDisconnect)(opts.autoReconnect);
		if (opts.autoReconnect) hub.connect(requestWebsocketURI(node, opts.verbose));
	});

	hub.onMessage([&](WebSocket<false> *ws, char *msg, size_t length, OpCode op) {
		std::cout << std::string(msg, length) << std::endl;
	});

	// register solution callback using an Async so that it's called on this thread
	std::function<void(uS::Async *)> onSolution = [&](uS::Async *a) {
		std::optional<Solution> solution = state->popSolutionImmediately();

		if (solution) {
			static long id = 1;
			static Json::StreamWriter *writer = Json::StreamWriterBuilder().newStreamWriter();

			Json::Value root;
			root["type"] = "submit_block";
			root["id"] = id;
			root["address"] = solution->address;
			root["nonce"] = solution->nonce;

			std::ostringstream ss;
			writer->write(root, &ss);

			hubClient->broadcast(ss.str().data(), ss.str().size(), TEXT);

			if (opts.onSubmitted) (*opts.onSubmitted)(*solution);

			id++;
		}
	};

	uS::Async solutionAsync(hub.getLoop());
	solutionAsync.setData(&onSolution);
	solutionAsync.start([](uS::Async *a) { (*reinterpret_cast<std::function<void(uS::Async *)> *>(a->getData()))(a); });

	// start a new thread that triggers the Async
	std::thread solutionChecker([&] {
		while (!state->isStopped()) {
			{
				// block if we're already waiting for a reply about a solution to prevent spam
				std::unique_lock lock(submitMtx);
				if (submitWaitingID) submitCV.wait(lock, [&] { return !submitWaitingID; });
			}

			state->waitForSolution();
			solutionAsync.send();
		}
	});

	std::cout << std::this_thread::get_id() << std::endl;

	hub.connect(requestWebsocketURI(node, opts.verbose));
	hub.run();
	solutionChecker.join();
}
