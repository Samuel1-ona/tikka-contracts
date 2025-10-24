// SPDX-License-Identifier: MIT
pragma solidity ^0.8.13;

import "forge-std/Script.sol";
import "../src/Raffle.sol";

contract DeployRaffle is Script {
    function run() external {
        uint256 deployerPrivateKey = vm.envUint("PRIVATE_KEY");
        vm.startBroadcast(deployerPrivateKey);

        // VRF parameters for Base Sepolia
        address vrfCoordinator = 0x5C210eF41CD1a72de73bF76eC39637bB0d3d7BEE; // Base Sepolia VRF Coordinator
        // Your actual subscription ID (now supports uint256)
        uint256 subscriptionId = 104463245950711925848002979325066164966376483155872594786291361268349330657388;
        bytes32 keyHash = 0x9e1344a1247c8a1785d0a4681a27152bffdb43666ae5bf7d14d24a5efd44bf71; // 30 gwei Key Hash
        uint32 callbackGasLimit = 200000; // Within the 2,500,000 max gas limit
        uint16 requestConfirmations = 3; // Within the 0-200 range
        
        Raffle raffle = new Raffle(
            vrfCoordinator,
            subscriptionId,
            keyHash,
            callbackGasLimit,
            requestConfirmations
        );

        console.log("Raffle contract deployed at:", address(raffle));
        console.log("Platform owner:", raffle.platformOwner());
        console.log("Platform service charge:", raffle.getPlatformServiceCharge(), "%");

        vm.stopBroadcast();
    }
}
