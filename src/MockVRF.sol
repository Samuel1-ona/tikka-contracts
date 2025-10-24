// SPDX-License-Identifier: MIT
pragma solidity ^0.8.13;

import "forge-std/console.sol";

/**
 * @title MockVRFCoordinatorV2Plus
 * @dev Mock VRF Coordinator for testing purposes
 */
contract MockVRFCoordinatorV2Plus {
    struct RandomWordsRequest {
        bytes32 keyHash;
        uint256 subId;
        uint16 requestConfirmations;
        uint32 callbackGasLimit;
        uint32 numWords;
        bytes extraArgs;
    }

    event RandomWordsRequested(
        uint256 indexed requestId,
        address indexed requester,
        RandomWordsRequest request
    );

    event RandomWordsFulfilled(
        uint256 indexed requestId,
        uint256[] randomWords
    );

    uint256 public nextRequestId = 1;
    mapping(uint256 => RandomWordsRequest) public requests;
    mapping(uint256 => address) public requestToRequester;

    function requestRandomWords(
        RandomWordsRequest memory request
    ) external returns (uint256 requestId) {
        requestId = nextRequestId++;
        requests[requestId] = request;
        requestToRequester[requestId] = msg.sender;

        emit RandomWordsRequested(requestId, msg.sender, request);
        return requestId;
    }

    function fulfillRandomWords(
        uint256 requestId,
        uint256[] memory randomWords
    ) external {
        address requester = requestToRequester[requestId];
        require(requester != address(0), "Invalid request ID");

        // Call the fulfillRandomWords function on the requester contract
        (bool success, ) = requester.call(
            abi.encodeWithSignature(
                "rawFulfillRandomWords(uint256,uint256[])",
                requestId,
                randomWords
            )
        );
        require(success, "Fulfillment failed");

        emit RandomWordsFulfilled(requestId, randomWords);
    }

    function generateRandomWords(uint256 count) external view returns (uint256[] memory) {
        uint256[] memory words = new uint256[](count);
        for (uint256 i = 0; i < count; i++) {
            words[i] = uint256(keccak256(abi.encodePacked(block.timestamp, block.prevrandao, i)));
        }
        return words;
    }
}

/**
 * @title VRFConsumerBaseV2Plus
 * @dev Base contract for VRF consumers
 */
abstract contract VRFConsumerBaseV2Plus {
    MockVRFCoordinatorV2Plus public s_vrfCoordinator;

    constructor(address _vrfCoordinator) {
        s_vrfCoordinator = MockVRFCoordinatorV2Plus(_vrfCoordinator);
    }

    function rawFulfillRandomWords(uint256 requestId, uint256[] memory randomWords) external {
        require(msg.sender == address(s_vrfCoordinator), "Only coordinator can fulfill");
        fulfillRandomWords(requestId, randomWords);
    }

    function fulfillRandomWords(uint256 requestId, uint256[] memory randomWords) internal virtual;
}

/**
 * @title VRFV2PlusClient
 * @dev Client interface for VRF v2.5
 */
library VRFV2PlusClient {
    struct RandomWordsRequest {
        bytes32 keyHash;
        uint256 subId;
        uint16 requestConfirmations;
        uint32 callbackGasLimit;
        uint32 numWords;
        bytes extraArgs;
    }

    struct ExtraArgsV1 {
        bool nativePayment;
    }

    function _argsToBytes(ExtraArgsV1 memory extraArgs) internal pure returns (bytes memory) {
        return abi.encode(extraArgs);
    }
}
