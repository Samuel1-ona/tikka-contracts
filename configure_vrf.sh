#!/bin/bash

# Configure VRF for deployed Raffle contract
# Contract Address: 0xed32402c968d04D1d7F6B3DEfcB7A91321736156

echo "Configuring VRF for Raffle contract..."

# Your actual subscription ID (the large one)
SUBSCRIPTION_ID="104463245950711925848002979325066164966376483155872594786291361268349330657388"

# VRF parameters for Base Sepolia
KEY_HASH="0x9e1344a1247c8a1785d0a4681a27152bffdb43666ae5bf7d14d24a5efd44bf71"
CALLBACK_GAS_LIMIT="200000"
REQUEST_CONFIRMATIONS="3"

# Contract address
CONTRACT_ADDRESS="0xed32402c968d04D1d7F6B3DEfcB7A91321736156"

# RPC URL
RPC_URL="https://base-sepolia.infura.io/v3/2DmS9CrnVeU2Caun612yGaPQ2aq"

echo "Calling configureVRF function..."
echo "Subscription ID: $SUBSCRIPTION_ID"
echo "Key Hash: $KEY_HASH"
echo "Callback Gas Limit: $CALLBACK_GAS_LIMIT"
echo "Request Confirmations: $REQUEST_CONFIRMATIONS"

# Call the configureVRF function
cast send $CONTRACT_ADDRESS \
    "configureVRF(uint64,bytes32,uint32,uint16)" \
    $SUBSCRIPTION_ID \
    $KEY_HASH \
    $CALLBACK_GAS_LIMIT \
    $REQUEST_CONFIRMATIONS \
    --rpc-url $RPC_URL \
    --private-key $PRIVATE_KEY

echo "VRF configuration completed!"
