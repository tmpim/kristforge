#pragma once

#include <mutex>
#include <condition_variable>
#include <optional>
#include <atomic>
#include <queue>
#include <iostream>

namespace kristforge {
	/** A target to mine for */
	struct Target {
	public:
		Target(std::string prevBlock, long work) : prevBlock(std::move(prevBlock)), work(work) {
			if (this->prevBlock.size() != 12) {
				throw std::range_error("Previous block length must equal 12");
			}
		}

		/** Short hash of the previous block */
		std::string prevBlock;

		/** Work value */
		long work;

		/** Compares two targets for equality */
		inline bool operator==(const Target &other) const { return prevBlock == other.prevBlock && work == other.work; }

		/** Compares two targets for inequality */
		inline bool operator!=(const Target &other) const { return prevBlock != other.prevBlock || work != other.work; }
	};

	inline std::ostream &operator<<(std::ostream &os, const Target &tgt) {
		return os << "Target (block " << tgt.prevBlock << " work " << std::to_string(tgt.work) << ")";
	}

	/** A solution for a specific target */
	struct Solution {
	public:
		Solution(Target target, std::string address, std::string nonce) :
				target(std::move(target)), address(std::move(address)), nonce(std::move(nonce)) {}

		/** The target that this solution applies to */
		Target target;

		/** The address this solution is valid for */
		std::string address;

		/** The nonce of this solution */
		std::string nonce;

		/** Compares two solutions for equality */
		inline bool operator==(const Solution &other) const {
			return target == other.target &&
			       address == other.address &&
			       nonce == other.nonce;
		}

		/** Compares two solutions for inequality */
		inline bool operator!=(const Solution &other) const {
			return target != other.target ||
			       address != other.address ||
			       nonce != other.nonce;
		}
	};

	inline std::ostream &operator<<(std::ostream &os, const Solution &sol) {
		return os << "Solution (address " << sol.address << " nonce " << sol.nonce << " " << sol.target << ")";
	}

	/** A shared mining state, used to synchronize mining tasks */
	class State {
	public:
		explicit State(std::string address) : address(std::move(address)) {
			if (this->address.size() != 10) throw std::range_error("Address length must be 10");
		}

		State(const State &) = delete;

		State &operator=(const State &) = delete;

		/** Gets the mining target, blocking until one is available if necessary */
		Target getTarget();

		/** Gets the target immediately, regardless of whether it's set or not */
		std::optional<Target> getTargetNow();

		/** Sets the current mining target */
		void setTarget(Target newTarget);

		/** Unsets the mining target */
		void unsetTarget();

		/** Clears all queued solutions */
		void clearSolutions();

		/** Add a solution to the end of the queue */
		void pushSolution(const Solution &solution);

		/** Pops the first solution immediately, regardless of whether one's available or not */
		std::optional<Solution> popSolutionImmediately();

		/** Pops the first solution off of the queue, blocking until one is available if necessary */
		Solution popSolution();

		/** Sets the stopped flag, signalling threads to exit */
		inline void stop() { stopped = true; }

		/** Checks whether the stop flag is currently set */
		inline bool isStopped() { return stopped; }

		/** The krist address to mine for */
		const std::string address;

		/** Total hashes evaluated */
		std::atomic<long> hashesCompleted;

	private:
		std::mutex targetMutex;
		std::condition_variable targetCV;
		std::optional<Target> target;

		std::mutex solutionMutex;
		std::condition_variable solutionCV;
		std::queue<Solution> solutions;

		std::atomic<bool> stopped = false;
	};
}