// SPDX-License-Identifier: MIT
pragma solidity ^0.8.13;

import "forge-std/Script.sol";
import "../src/Raffle.sol";

contract ConfigureVRF is Script {
    function run() external {
        uint256 deployerPrivateKey = vm.envUint("PRIVATE_KEY");
        vm.startBroadcast(deployerPrivateKey);

        // Your deployed contract address (latest deployment with receive() function)
        address payable raffleAddress = payable(0x60fd4f42B818b173d7252859963c7131Ed68CA6D);
        Raffle raffle = Raffle(raffleAddress);

        // Your actual subscription ID (now supports uint256)
        uint256 subscriptionId = 104463245950711925848002979325066164966376483155872594786291361268349330657388;
        
        // VRF parameters for Base Sepolia
        bytes32 keyHash = 0x9e1344a1247c8a1785d0a4681a27152bffdb43666ae5bf7d14d24a5efd44bf71;
        uint32 callbackGasLimit = 200000;
        uint16 requestConfirmations = 3;

        console.log("Configuring VRF for contract:", raffleAddress);
        console.log("Subscription ID:", subscriptionId);
        console.log("Key Hash:", vm.toString(keyHash));
        console.log("Callback Gas Limit:", callbackGasLimit);
        console.log("Request Confirmations:", requestConfirmations);

        // Call configureVRF function (now supports uint256 subscription ID)
        raffle.configureVRF(
            subscriptionId,
            keyHash,
            callbackGasLimit,
            requestConfirmations
        );

        console.log("VRF configuration completed successfully!");

        vm.stopBroadcast();
    }
}
