# Raffle Contract System

A comprehensive smart contract system for managing raffles on Ethereum, built with Solidity and Foundry.

## Features

### Core Functionality

-   **Raffle Creation**: Create raffles with custom parameters
-   **Multi-Token Support**: Use ETH or any ERC20 token for ticket purchases
-   **Ticket Purchasing**: Buy single or multiple tickets with ETH or ERC20 tokens
-   **Prize Management**: Support for ETH, ERC20 tokens, and ERC721 NFTs as prizes
-   **Escrow System**: Secure prize holding until raffle completion
-   **Winner Selection**: Platform owner can select winners
-   **Automatic Prize Distribution**: Anyone can finalize raffles to transfer prizes to winners
-   **Platform Fees**: Configurable service charge system

### Raffle Parameters

-   **Description**: Custom description for each raffle
-   **End Time**: Unix timestamp when raffle ends
-   **Max Tickets**: Maximum number of tickets that can be sold
-   **Allow Multiple Tickets**: Whether users can buy multiple tickets
-   **Ticket Price**: Price per ticket (in wei for ETH, or token units for ERC20)
-   **Ticket Token**: Token to use for purchases (address(0) for ETH, otherwise ERC20 address)

## Contract Functions

### Raffle Management

-   `createRaffle()` - Create a new raffle
-   `getRaffleData()` - Get complete raffle information
-   `isRaffleActive()` - Check if raffle is still active
-   `getTotalRaffles()` - Get total number of raffles created

### Ticket Operations

-   `buyTicket()` - Buy a single ticket
-   `buyMultipleTickets()` - Buy multiple tickets at once
-   `getUserTicketsInRaffle()` - Get user's ticket count for a raffle
-   `getUserTicketIds()` - Get all ticket IDs for a user
-   `getTicketData()` - Get ticket information by ID
-   `getRaffleTicketIds()` - Get all ticket IDs for a raffle

### Winner Management

-   `selectWinner()` - Select winner for a raffle (platform owner only)
-   `withdrawWinnings()` - Withdraw prize money (winner only)
-   `finalizeRaffle()` - Transfer prize to winner (anyone can call)

### Prize Management

-   `depositPrizeETH()` - Deposit ETH as prize (raffle creator only)
-   `depositPrizeToken()` - Deposit ERC20 tokens as prize (raffle creator only)
-   `depositPrizeNFT()` - Deposit ERC721 NFT as prize (raffle creator only)
-   `getPrizeData()` - Get prize information for a raffle
-   `hasPrizeDeposited()` - Check if prize has been deposited

### Platform Management

-   `setPlatformServiceCharge()` - Set service charge percentage (platform owner only)
-   `getPlatformServiceCharge()` - Get current service charge
-   `getContractBalance()` - Get total contract balance

## Events

-   `RaffleCreated` - Emitted when a new raffle is created
-   `TicketPurchased` - Emitted when a ticket is purchased
-   `WinnerSelected` - Emitted when a winner is selected
-   `WinningsWithdrawn` - Emitted when winnings are withdrawn
-   `PrizeDeposited` - Emitted when a prize is deposited
-   `PrizeWithdrawn` - Emitted when a prize is transferred to winner
-   `RaffleFinalized` - Emitted when a raffle is finalized

## Usage Examples

### Creating a Raffle

```solidity
// Create ETH raffle
raffle.createRaffle(
    "Weekly NFT Raffle",           // description
    block.timestamp + 7 days,      // end time
    100,                          // max tickets
    true,                         // allow multiple tickets
    0.1 ether,                    // ticket price
    address(0)                    // use ETH for tickets
);

// Create ERC20 token raffle
raffle.createRaffle(
    "Token Raffle",               // description
    block.timestamp + 7 days,      // end time
    100,                          // max tickets
    true,                         // allow multiple tickets
    100 ether,                    // ticket price (100 tokens)
    tokenAddress                  // use ERC20 token for tickets
);
```

### Buying Tickets

```solidity
// Buy single ticket with ETH
raffle.buyTicket{value: 0.1 ether}(raffleId);

// Buy multiple tickets with ETH
raffle.buyMultipleTickets{value: 0.3 ether}(raffleId, 3);

// Buy tickets with ERC20 tokens (approve first)
token.approve(address(raffle), 100 ether);
raffle.buyTicket(raffleId); // No ETH needed for token raffles
```

### Depositing Prizes

```solidity
// Deposit ETH as prize
raffle.depositPrizeETH{value: 1 ether}(raffleId);

// Deposit ERC20 tokens as prize
token.approve(address(raffle), 1000 ether);
raffle.depositPrizeToken(raffleId, tokenAddress, 1000 ether);

// Deposit NFT as prize
nft.approve(address(raffle), tokenId);
raffle.depositPrizeNFT(raffleId, nftAddress, tokenId);
```

### Selecting Winner and Finalizing

```solidity
// Platform owner selects winner
raffle.selectWinner(raffleId, winningTicketId);

// Anyone can finalize raffle to transfer prize to winner
raffle.finalizeRaffle(raffleId);

// Winner can also withdraw ticket sale proceeds (minus platform fee)
raffle.withdrawWinnings(raffleId);
```

## Security Features

-   **Access Control**: Only platform owner can select winners and set service charges
-   **Time Validation**: Raffles must end in the future
-   **Ticket Limits**: Enforces maximum ticket limits
-   **Multiple Ticket Control**: Respects allowMultipleTickets setting
-   **Winner Validation**: Only actual winners can withdraw prizes
-   **Single Withdrawal**: Prevents double withdrawal of winnings

## Platform Economics

-   **Service Charge**: Configurable percentage (default 5%, max 20%)
-   **Prize Distribution**: Winner gets total prize pool minus service charge
-   **Platform Revenue**: Service charge goes to platform owner

## Testing

The contract includes comprehensive tests covering:

-   Raffle creation with various parameters
-   Ticket purchasing (single and multiple)
-   Edge cases and error conditions
-   Winner selection and withdrawal
-   Platform fee calculations
-   Complete raffle lifecycle

Run tests with:

```bash
forge test --match-contract RaffleTest
```

## Deployment

Deploy the contract using the provided script:

```bash
forge script script/DeployRaffle.s.sol --rpc-url <RPC_URL> --private-key <PRIVATE_KEY> --broadcast
```

## Contract Architecture

### Data Structures

-   `RaffleData`: Complete raffle information
-   `Ticket`: Individual ticket data
-   Mappings for efficient data retrieval

### State Management

-   Raffle lifecycle tracking
-   Ticket ownership and status
-   Winner selection and withdrawal status
-   Platform configuration

## Future Enhancements

-   Random winner selection using Chainlink VRF
-   Automated raffle ending
-   Raffle categories and filtering
-   Advanced analytics and reporting
-   Multi-token support
-   Governance features

## Deployment

### Base Sepolia Testnet

The contract is configured for deployment on Base Sepolia testnet with the following VRF parameters:

-   **VRF Coordinator**: `0x5C210eF41CD1a72de73bF76eC39637bB0d3d7BEE`
-   **LINK Token**: `0xE4aB69C077896252FAFBD49EFD26B5D171A32410`
-   **Key Hash**: `0x9e1344a1247c8a1785d0a4681a27152bffdb43666ae5bf7d14d24a5efd44bf71`

### Deployment Steps

1. **Setup Environment**:

    ```bash
    # Create .env file
    PRIVATE_KEY=your_private_key_here
    BASE_SEPOLIA_RPC_URL=https://sepolia.base.org
    ETHERSCAN_API_KEY=your_etherscan_api_key_here
    ```

2. **Create VRF Subscription**:

    - Go to [Chainlink VRF Subscription Manager](https://vrf.chain.link/base-sepolia)
    - Create subscription and fund with LINK tokens
    - Update `subscriptionId` in deployment script

3. **Deploy Contract**:

    ```bash
    forge script script/DeployRaffle.s.sol --rpc-url $BASE_SEPOLIA_RPC_URL --broadcast --verify
    ```

4. **Add Consumer**:
    - Add deployed contract as consumer to VRF subscription
    - This enables the contract to request random numbers

### Deployed Contract Details

**Base Sepolia Deployment (Latest - with receive() function):**

-   **Contract Address**: `0x60fd4f42B818b173d7252859963c7131Ed68CA6D`
-   **Deployer**: `0xF18ca72961b486318551B827F6A7124cF1caDf81`
-   **Transaction Hash**: `0x4f6735f049ca6025af24dad02385a4ddfdcb702f02f5eea7e4a53be6ecfd599b`
-   **Explorer**: [Base Sepolia Explorer](https://sepolia.basescan.org/address/0x60fd4f42B818b173d7252859963c7131Ed68CA6D)
-   **Features**: Supports large subscription IDs (uint256), native ETH payments for VRF, receive() function for ETH transfers

**Previous Deployment (with uint256 support):**

-   **Contract Address**: `0x69A2F4DeC343B06956738376f07dca1787B342C5`
-   **Deployer**: `0xF18ca72961b486318551B827F6A7124cF1caDf81`
-   **Transaction Hash**: `0x065233905b0d633f974406f671c3a77b1b9e609122f838d2f0e855413f16fa51`
-   **Explorer**: [Base Sepolia Explorer](https://sepolia.basescan.org/address/0x69A2F4DeC343B06956738376f07dca1787B342C5)
-   **Features**: Supports large subscription IDs (uint256), native ETH payments for VRF

**Previous Deployment (Legacy):**

-   **Contract Address**: `0xed32402c968d04D1d7F6B3DEfcB7A91321736156`
-   **Deployer**: `0xF18ca72961b486318551B827F6A7124cF1caDf81`
-   **Transaction Hash**: `0x6a5bf547fea8c67db991f1db74c03484f8b09532112c7a6d2e45fe366c043cb9`
-   **Explorer**: [Base Sepolia Explorer](https://sepolia.basescan.org/address/0xed32402c968d04D1d7F6B3DEfcB7A91321736156)

**Important**: After deployment, you need to:

1. Call `configureVRF` with your actual subscription ID
2. Add the contract as a consumer to your VRF subscription
3. Fund the contract with ETH for VRF requests

See `DEPLOYMENT_GUIDE.md` for detailed deployment instructions.

## License

MIT License - see LICENSE file for details.
