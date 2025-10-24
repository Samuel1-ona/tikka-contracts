// SPDX-License-Identifier: MIT
pragma solidity ^0.8.13;

import "forge-std/Script.sol";

contract CreateNewSubscription is Script {
    function run() external {
        uint256 deployerPrivateKey = vm.envUint("PRIVATE_KEY");
        vm.startBroadcast(deployerPrivateKey);

        console.log("=== VRF Subscription Setup Instructions ===");
        console.log("");
        console.log("Your current subscription ID is too large for uint64.");
        console.log("You need to create a new subscription with a smaller ID.");
        console.log("");
        console.log("Steps:");
        console.log("1. Go to: https://vrf.chain.link/base-sepolia");
        console.log("2. Connect your wallet");
        console.log("3. Click 'Create Subscription'");
        console.log("4. The new subscription will have a smaller ID (like 1, 2, 3, etc.)");
        console.log("5. Use the new subscription ID in your contract");
        console.log("");
        console.log("After creating the new subscription:");
        console.log("- Update the deployment script with the new ID");
        console.log("- Redeploy the contract with the correct subscription ID");
        console.log("- Add the contract as a consumer to the new subscription");

        vm.stopBroadcast();
    }
}
