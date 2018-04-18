#pragma once

#include "state.h"
#include <memory>
#include <functional>

namespace kristforge::network {
	/** Extra options for the network runner */
	struct Options {
		/** If set, will automatically attempt to reconnect if connection is dropped */
		bool autoReconnect = false;

		bool verbose = false;

		/** A callback for when a connection is successfully established (or reestablished) */
		std::optional<std::function<void()>> onConnect;

		/** A callback for when a connection is disconnected - parameter is true if a reconnection is being attempted */
		std::optional<std::function<void(bool)>> onDisconnect;

		/** A callback for when a solution is submitted */
		std::optional<std::function<void(kristforge::Solution)>> onSubmitted;

		/** A callback for when a solution is accepted */
		std::optional<std::function<void(kristforge::Solution)>> onSolved;

		/** A callback for when a solution is rejected */
		std::optional<std::function<void(kristforge::Solution)>> onRejected;
	};

	/** Connects to the node and synchronously sets mining target and submits solutions */
	void run(const std::string &node, const std::shared_ptr<State> &state, Options opts = Options());
}