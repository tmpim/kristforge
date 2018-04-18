#include "state.h"

kristforge::Target kristforge::State::getTarget() {
	std::unique_lock<std::mutex> lock(targetMutex);

	if (!target) {
		targetCV.wait(lock, [&] { return target; });
	}

	return *target;
}

std::optional<kristforge::Target> kristforge::State::getTargetNow() {
	std::unique_lock<std::mutex> lock(targetMutex);
	return target;
}

void kristforge::State::setTarget(kristforge::Target newTarget) {
	std::lock_guard lock(targetMutex);

	if (!target || *target != newTarget) {
		target = newTarget;
		targetCV.notify_all();

		clearSolutions();
	}
}

void kristforge::State::unsetTarget() {
	std::lock_guard lock(targetMutex);

	if (target) {
		target.reset();
		targetCV.notify_all();

		clearSolutions();
	}
}

void kristforge::State::clearSolutions() {
	std::lock_guard lock(solutionMutex);
	solutions = {};
	solutionCV.notify_all();
}

void kristforge::State::pushSolution(const kristforge::Solution &solution) {
	std::lock_guard lock(solutionMutex);
	solutions.push(solution);
	solutionCV.notify_all();
}

std::optional<kristforge::Solution> kristforge::State::popSolutionImmediately() {
	std::lock_guard lock(solutionMutex);

	if (solutions.empty()) {
		return {};
	} else {
		Solution ret = solutions.front();
		solutions.pop();
		return ret;
	}
}

kristforge::Solution kristforge::State::popSolution() {
	std::unique_lock<std::mutex> lock(solutionMutex);

	if (solutions.empty()) {
		solutionCV.wait(lock, [&] { return !solutions.empty(); });
	}

	Solution ret = solutions.front();
	solutions.pop();
	return ret;
}
