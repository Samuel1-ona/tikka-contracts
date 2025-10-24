// SPDX-License-Identifier: MIT
pragma solidity ^0.8.13;

import "forge-std/console.sol";
import "forge-std/interfaces/IERC20.sol";
import "forge-std/interfaces/IERC721.sol";
import "./MockVRF.sol";

contract Raffle is VRFConsumerBaseV2Plus {
    // Events
    event RaffleCreated(uint256 indexed raffleId, address indexed creator, string description, uint256 endTime, uint256 maxTickets, bool allowMultipleTickets);
    event TicketPurchased(uint256 indexed raffleId, address indexed buyer, uint256 ticketId, uint256 amount);
    event WinnerSelected(uint256 indexed raffleId, address indexed winner, uint256 ticketId);
    event WinningsWithdrawn(uint256 indexed raffleId, address indexed winner, uint256 amount);
    event PrizeDeposited(uint256 indexed raffleId, address indexed depositor, address token, uint256 tokenId, uint256 amount);
    event PrizeWithdrawn(uint256 indexed raffleId, address indexed winner, address token, uint256 tokenId, uint256 amount);
    event RaffleFinalized(uint256 indexed raffleId, address indexed winner);
    event RandomWinnerRequested(uint256 indexed raffleId, uint256 indexed requestId);
    event VRFConfigurationUpdated(uint256 subscriptionId, bytes32 keyHash, uint32 callbackGasLimit, uint16 requestConfirmations);

    // Structs
    struct RaffleData {
        uint256 id;
        address creator;
        string description;
        uint256 endTime;
        uint256 maxTickets;
        bool allowMultipleTickets;
        uint256 ticketPrice;
        address ticketToken; // Address(0) for ETH, otherwise ERC20 token
        uint256 totalTicketsSold;
        bool isActive;
        address winner;
        uint256 winningTicketId;
        bool winningsWithdrawn;
        bool isFinalized;
    }

    struct PrizeData {
        address token; // Address(0) for ETH, otherwise ERC20/ERC721 token
        uint256 tokenId; // For ERC721, 0 for ERC20/ETH
        uint256 amount; // For ERC20/ETH, 0 for ERC721
        bool isNFT; // true for ERC721, false for ERC20/ETH
        bool isDeposited;
    }

    struct Ticket {
        uint256 id;
        uint256 raffleId;
        address owner;
        bool isWinner;
        uint256 purchaseTime;
    }

    // State variables
    uint256 public nextRaffleId = 1;
    uint256 public nextTicketId = 1;
    uint256 public platformServiceCharge = 5; // 5% service charge
    address public platformOwner;
    
    // VRF v2.5 variables
    uint256 public s_subscriptionId;
    bytes32 public s_keyHash;
    uint32 public s_callbackGasLimit;
    uint16 public s_requestConfirmations;
    uint32 public s_numWords = 1;
    
    // VRF request tracking
    mapping(uint256 => uint256) public requestIdToRaffleId; // requestId => raffleId
    mapping(uint256 => bool) public pendingVRFRequests; // raffleId => has pending request
    
    // Mappings
    mapping(uint256 => RaffleData) public raffles;
    mapping(uint256 => PrizeData) public prizes; // raffleId => prize data
    mapping(uint256 => Ticket) public tickets;
    mapping(uint256 => mapping(address => uint256)) public userTicketsInRaffle; // raffleId => user => ticket count
    mapping(address => uint256[]) public userTickets; // user => array of ticket IDs
    mapping(uint256 => uint256[]) public raffleTickets; // raffleId => array of ticket IDs

    // Modifiers
    modifier onlyPlatformOwner() {
        require(msg.sender == platformOwner, "Only platform owner");
        _;
    }

    modifier raffleExists(uint256 _raffleId) {
        require(_raffleId > 0 && _raffleId < nextRaffleId, "Raffle does not exist");
        _;
    }

    modifier raffleActive(uint256 _raffleId) {
        require(raffles[_raffleId].isActive, "Raffle is not active");
        require(block.timestamp < raffles[_raffleId].endTime, "Raffle has ended");
        _;
    }

    constructor(
        address _vrfCoordinator,
        uint256 _subscriptionId,
        bytes32 _keyHash,
        uint32 _callbackGasLimit,
        uint16 _requestConfirmations
    ) VRFConsumerBaseV2Plus(_vrfCoordinator) {
        platformOwner = msg.sender;
        s_subscriptionId = _subscriptionId;
        s_keyHash = _keyHash;
        s_callbackGasLimit = _callbackGasLimit;
        s_requestConfirmations = _requestConfirmations;
    }

    /**
     * @dev Create a new raffle
     * @param _description Description of the raffle
     * @param _endTime Unix timestamp when raffle ends
     * @param _maxTickets Maximum number of tickets that can be sold
     * @param _allowMultipleTickets Whether users can buy multiple tickets
     * @param _ticketPrice Price per ticket
     * @param _ticketToken Token to use for ticket purchases (address(0) for ETH)
     */
    function createRaffle(
        string memory _description,
        uint256 _endTime,
        uint256 _maxTickets,
        bool _allowMultipleTickets,
        uint256 _ticketPrice,
        address _ticketToken
    ) external {
        require(_endTime > block.timestamp, "End time must be in the future");
        require(_maxTickets > 0, "Max tickets must be greater than 0");
        require(_ticketPrice > 0, "Ticket price must be greater than 0");

        uint256 raffleId = nextRaffleId++;
        
        raffles[raffleId] = RaffleData({
            id: raffleId,
            creator: msg.sender,
            description: _description,
            endTime: _endTime,
            maxTickets: _maxTickets,
            allowMultipleTickets: _allowMultipleTickets,
            ticketPrice: _ticketPrice,
            ticketToken: _ticketToken,
            totalTicketsSold: 0,
            isActive: true,
            winner: address(0),
            winningTicketId: 0,
            winningsWithdrawn: false,
            isFinalized: false
        });

        emit RaffleCreated(raffleId, msg.sender, _description, _endTime, _maxTickets, _allowMultipleTickets);
    }

    /**
     * @dev Buy a single ticket for a raffle
     * @param _raffleId ID of the raffle
     */
    function buyTicket(uint256 _raffleId) external payable raffleExists(_raffleId) raffleActive(_raffleId) {
        RaffleData storage raffle = raffles[_raffleId];
        
        require(raffle.totalTicketsSold < raffle.maxTickets, "No tickets available");
        
        // Check if user already has tickets and if multiple tickets are allowed
        if (!raffle.allowMultipleTickets) {
            require(userTicketsInRaffle[_raffleId][msg.sender] == 0, "Multiple tickets not allowed");
        }

        // Handle payment based on token type
        if (raffle.ticketToken == address(0)) {
            // ETH payment
            require(msg.value == raffle.ticketPrice, "Incorrect ticket price");
        } else {
            // ERC20 token payment
            require(msg.value == 0, "ETH not accepted for token raffles");
            IERC20 token = IERC20(raffle.ticketToken);
            require(token.transferFrom(msg.sender, address(this), raffle.ticketPrice), "Token transfer failed");
        }

        // Create ticket
        uint256 ticketId = nextTicketId++;
        tickets[ticketId] = Ticket({
            id: ticketId,
            raffleId: _raffleId,
            owner: msg.sender,
            isWinner: false,
            purchaseTime: block.timestamp
        });

        // Update mappings
        userTickets[msg.sender].push(ticketId);
        raffleTickets[_raffleId].push(ticketId);
        userTicketsInRaffle[_raffleId][msg.sender]++;
        raffle.totalTicketsSold++;

        emit TicketPurchased(_raffleId, msg.sender, ticketId, raffle.ticketPrice);
    }

    /**
     * @dev Buy multiple tickets for a raffle
     * @param _raffleId ID of the raffle
     * @param _quantity Number of tickets to buy
     */
    function buyMultipleTickets(uint256 _raffleId, uint256 _quantity) external payable raffleExists(_raffleId) raffleActive(_raffleId) {
        RaffleData storage raffle = raffles[_raffleId];
        
        require(_quantity > 0, "Quantity must be greater than 0");
        require(raffle.totalTicketsSold + _quantity <= raffle.maxTickets, "Not enough tickets available");
        
        // Check if user already has tickets and if multiple tickets are allowed
        if (!raffle.allowMultipleTickets) {
            require(userTicketsInRaffle[_raffleId][msg.sender] == 0, "Multiple tickets not allowed");
        }

        uint256 totalPrice = raffle.ticketPrice * _quantity;

        // Handle payment based on token type
        if (raffle.ticketToken == address(0)) {
            // ETH payment
            require(msg.value == totalPrice, "Incorrect total amount");
        } else {
            // ERC20 token payment
            require(msg.value == 0, "ETH not accepted for token raffles");
            IERC20 token = IERC20(raffle.ticketToken);
            require(token.transferFrom(msg.sender, address(this), totalPrice), "Token transfer failed");
        }

        // Create multiple tickets
        for (uint256 i = 0; i < _quantity; i++) {
            uint256 ticketId = nextTicketId++;
            tickets[ticketId] = Ticket({
                id: ticketId,
                raffleId: _raffleId,
                owner: msg.sender,
                isWinner: false,
                purchaseTime: block.timestamp
            });

            userTickets[msg.sender].push(ticketId);
            raffleTickets[_raffleId].push(ticketId);
            
            emit TicketPurchased(_raffleId, msg.sender, ticketId, raffle.ticketPrice);
        }

        userTicketsInRaffle[_raffleId][msg.sender] += _quantity;
        raffle.totalTicketsSold += _quantity;
    }

    /**
     * @dev Get raffle data
     * @param _raffleId ID of the raffle
     */
    function getRaffleData(uint256 _raffleId) external view raffleExists(_raffleId) returns (RaffleData memory) {
        return raffles[_raffleId];
    }

    /**
     * @dev Get number of tickets a user has in a specific raffle
     * @param _raffleId ID of the raffle
     * @param _user Address of the user
     */
    function getUserTicketsInRaffle(uint256 _raffleId, address _user) external view raffleExists(_raffleId) returns (uint256) {
        return userTicketsInRaffle[_raffleId][_user];
    }

    /**
     * @dev Get all ticket IDs for a user
     * @param _user Address of the user
     */
    function getUserTicketIds(address _user) external view returns (uint256[] memory) {
        return userTickets[_user];
    }

    /**
     * @dev Get ticket data by ticket ID
     * @param _ticketId ID of the ticket
     */
    function getTicketData(uint256 _ticketId) external view returns (Ticket memory) {
        require(_ticketId > 0 && _ticketId < nextTicketId, "Ticket does not exist");
        return tickets[_ticketId];
    }

    /**
     * @dev Get all ticket IDs for a raffle
     * @param _raffleId ID of the raffle
     */
    function getRaffleTicketIds(uint256 _raffleId) external view raffleExists(_raffleId) returns (uint256[] memory) {
        return raffleTickets[_raffleId];
    }

    /**
     * @dev Request random winner selection for a raffle using Chainlink VRF
     * @param _raffleId ID of the raffle
     */
    function requestRandomWinner(uint256 _raffleId) external onlyPlatformOwner raffleExists(_raffleId) {
        RaffleData storage raffle = raffles[_raffleId];
        
        require(block.timestamp >= raffle.endTime, "Raffle has not ended yet");
        require(raffle.isActive, "Raffle is not active");
        require(raffleTickets[_raffleId].length > 0, "No tickets sold");
        require(!pendingVRFRequests[_raffleId], "VRF request already pending");

        // Mark that we have a pending VRF request for this raffle
        pendingVRFRequests[_raffleId] = true;

        // Request random words from VRF
        MockVRFCoordinatorV2Plus.RandomWordsRequest memory request = MockVRFCoordinatorV2Plus.RandomWordsRequest({
            keyHash: s_keyHash,
            subId: s_subscriptionId,
            requestConfirmations: s_requestConfirmations,
            callbackGasLimit: s_callbackGasLimit,
            numWords: s_numWords,
            extraArgs: VRFV2PlusClient._argsToBytes(VRFV2PlusClient.ExtraArgsV1({nativePayment: true}))
        });
        
        uint256 requestId = s_vrfCoordinator.requestRandomWords(request);

        // Track the request
        requestIdToRaffleId[requestId] = _raffleId;

        emit RandomWinnerRequested(_raffleId, requestId);
    }

    /**
     * @dev Callback function called by VRF Coordinator when random words are fulfilled
     * @param requestId The request ID from VRF
     * @param randomWords Array of random words returned by VRF
     */
    function fulfillRandomWords(uint256 requestId, uint256[] memory randomWords) internal override {
        uint256 raffleId = requestIdToRaffleId[requestId];
        require(raffleId > 0, "Invalid request ID");
        require(pendingVRFRequests[raffleId], "No pending request for this raffle");

        RaffleData storage raffle = raffles[raffleId];
        require(raffle.isActive, "Raffle is not active");

        // Clear pending request
        pendingVRFRequests[raffleId] = false;

        // Select winner using random number
        uint256[] memory ticketIds = raffleTickets[raffleId];
        uint256 randomIndex = randomWords[0] % ticketIds.length;
        uint256 winningTicketId = ticketIds[randomIndex];

        // Set winner
        raffle.isActive = false;
        raffle.winner = tickets[winningTicketId].owner;
        raffle.winningTicketId = winningTicketId;
        tickets[winningTicketId].isWinner = true;

        emit WinnerSelected(raffleId, raffle.winner, winningTicketId);
    }

    /**
     * @dev Withdraw winnings (only winner can call this)
     * @param _raffleId ID of the raffle
     */
    function withdrawWinnings(uint256 _raffleId) external raffleExists(_raffleId) {
        RaffleData storage raffle = raffles[_raffleId];
        
        require(msg.sender == raffle.winner, "Only winner can withdraw");
        require(!raffle.winningsWithdrawn, "Winnings already withdrawn");
        require(!raffle.isActive, "Raffle is still active");

        uint256 totalPrize = raffle.totalTicketsSold * raffle.ticketPrice;
        uint256 serviceCharge = (totalPrize * platformServiceCharge) / 100;
        uint256 winnerAmount = totalPrize - serviceCharge;

        raffle.winningsWithdrawn = true;

        // Transfer winnings to winner
        payable(msg.sender).transfer(winnerAmount);
        
        // Transfer service charge to platform owner
        payable(platformOwner).transfer(serviceCharge);

        emit WinningsWithdrawn(_raffleId, msg.sender, winnerAmount);
    }

    /**
     * @dev Set platform service charge percentage (only platform owner)
     * @param _newCharge New service charge percentage
     */
    function setPlatformServiceCharge(uint256 _newCharge) external onlyPlatformOwner {
        require(_newCharge <= 20, "Service charge cannot exceed 20%");
        platformServiceCharge = _newCharge;
    }

    /**
     * @dev Get platform service charge
     */
    function getPlatformServiceCharge() external view returns (uint256) {
        return platformServiceCharge;
    }

    /**
     * @dev Get total number of raffles created
     */
    function getTotalRaffles() external view returns (uint256) {
        return nextRaffleId - 1;
    }

    /**
     * @dev Check if a raffle is still active
     * @param _raffleId ID of the raffle
     */
    function isRaffleActive(uint256 _raffleId) external view raffleExists(_raffleId) returns (bool) {
        return raffles[_raffleId].isActive && block.timestamp < raffles[_raffleId].endTime;
    }

    /**
     * @dev Get contract balance
     */
    function getContractBalance() external view returns (uint256) {
        return address(this).balance;
    }

    /**
     * @dev Deposit prize for a raffle (ERC20 token)
     * @param _raffleId ID of the raffle
     * @param _token ERC20 token address
     * @param _amount Amount of tokens to deposit
     */
    function depositPrizeToken(uint256 _raffleId, address _token, uint256 _amount) external raffleExists(_raffleId) {
        RaffleData storage raffle = raffles[_raffleId];
        require(msg.sender == raffle.creator, "Only raffle creator can deposit prize");
        require(!raffle.isFinalized, "Raffle already finalized");
        require(!prizes[_raffleId].isDeposited, "Prize already deposited");

        IERC20 token = IERC20(_token);
        require(token.transferFrom(msg.sender, address(this), _amount), "Token transfer failed");

        prizes[_raffleId] = PrizeData({
            token: _token,
            tokenId: 0,
            amount: _amount,
            isNFT: false,
            isDeposited: true
        });

        emit PrizeDeposited(_raffleId, msg.sender, _token, 0, _amount);
    }

    /**
     * @dev Deposit prize for a raffle (ERC721 NFT)
     * @param _raffleId ID of the raffle
     * @param _token ERC721 token address
     * @param _tokenId NFT token ID
     */
    function depositPrizeNFT(uint256 _raffleId, address _token, uint256 _tokenId) external raffleExists(_raffleId) {
        RaffleData storage raffle = raffles[_raffleId];
        require(msg.sender == raffle.creator, "Only raffle creator can deposit prize");
        require(!raffle.isFinalized, "Raffle already finalized");
        require(!prizes[_raffleId].isDeposited, "Prize already deposited");

        IERC721 nft = IERC721(_token);
        require(nft.ownerOf(_tokenId) == msg.sender, "Not NFT owner");
        nft.transferFrom(msg.sender, address(this), _tokenId);

        prizes[_raffleId] = PrizeData({
            token: _token,
            tokenId: _tokenId,
            amount: 0,
            isNFT: true,
            isDeposited: true
        });

        emit PrizeDeposited(_raffleId, msg.sender, _token, _tokenId, 0);
    }

    /**
     * @dev Deposit ETH as prize
     * @param _raffleId ID of the raffle
     */
    function depositPrizeETH(uint256 _raffleId) external payable raffleExists(_raffleId) {
        RaffleData storage raffle = raffles[_raffleId];
        require(msg.sender == raffle.creator, "Only raffle creator can deposit prize");
        require(!raffle.isFinalized, "Raffle already finalized");
        require(!prizes[_raffleId].isDeposited, "Prize already deposited");
        require(msg.value > 0, "Amount must be greater than 0");

        prizes[_raffleId] = PrizeData({
            token: address(0),
            tokenId: 0,
            amount: msg.value,
            isNFT: false,
            isDeposited: true
        });

        emit PrizeDeposited(_raffleId, msg.sender, address(0), 0, msg.value);
    }

    /**
     * @dev Get prize data for a raffle
     * @param _raffleId ID of the raffle
     */
    function getPrizeData(uint256 _raffleId) external view raffleExists(_raffleId) returns (PrizeData memory) {
        return prizes[_raffleId];
    }

    /**
     * @dev Finalize raffle and transfer prize to winner
     * @param _raffleId ID of the raffle
     */
    function finalizeRaffle(uint256 _raffleId) external raffleExists(_raffleId) {
        RaffleData storage raffle = raffles[_raffleId];
        PrizeData storage prize = prizes[_raffleId];
        
        require(!raffle.isFinalized, "Raffle already finalized");
        require(raffle.winner != address(0), "No winner selected");
        require(prize.isDeposited, "No prize deposited");

        raffle.isFinalized = true;

        if (prize.isNFT) {
            // Transfer NFT to winner
            IERC721 nft = IERC721(prize.token);
            nft.transferFrom(address(this), raffle.winner, prize.tokenId);
            emit PrizeWithdrawn(_raffleId, raffle.winner, prize.token, prize.tokenId, 0);
        } else if (prize.token == address(0)) {
            // Transfer ETH to winner
            payable(raffle.winner).transfer(prize.amount);
            emit PrizeWithdrawn(_raffleId, raffle.winner, address(0), 0, prize.amount);
        } else {
            // Transfer ERC20 token to winner
            IERC20 token = IERC20(prize.token);
            require(token.transfer(raffle.winner, prize.amount), "Token transfer failed");
            emit PrizeWithdrawn(_raffleId, raffle.winner, prize.token, 0, prize.amount);
        }

        emit RaffleFinalized(_raffleId, raffle.winner);
    }

    /**
     * @dev Check if raffle has prize deposited
     * @param _raffleId ID of the raffle
     */
    function hasPrizeDeposited(uint256 _raffleId) external view raffleExists(_raffleId) returns (bool) {
        return prizes[_raffleId].isDeposited;
    }

    /**
     * @dev Get contract token balance
     * @param _token Token address
     */
    function getContractTokenBalance(address _token) external view returns (uint256) {
        if (_token == address(0)) {
            return address(this).balance;
        }
        return IERC20(_token).balanceOf(address(this));
    }

    // ============ GETTER FUNCTIONS ============

    /**
     * @dev Get all raffle IDs
     */
    function getAllRaffleIds() external view returns (uint256[] memory) {
        uint256[] memory raffleIds = new uint256[](nextRaffleId - 1);
        for (uint256 i = 1; i < nextRaffleId; i++) {
            raffleIds[i - 1] = i;
        }
        return raffleIds;
    }

    /**
     * @dev Get raffle IDs by creator
     * @param _creator Address of the raffle creator
     */
    function getRaffleIdsByCreator(address _creator) external view returns (uint256[] memory) {
        uint256 count = 0;
        // First pass: count matching raffles
        for (uint256 i = 1; i < nextRaffleId; i++) {
            if (raffles[i].creator == _creator) {
                count++;
            }
        }
        
        // Second pass: populate array
        uint256[] memory creatorRaffles = new uint256[](count);
        uint256 index = 0;
        for (uint256 i = 1; i < nextRaffleId; i++) {
            if (raffles[i].creator == _creator) {
                creatorRaffles[index] = i;
                index++;
            }
        }
        return creatorRaffles;
    }

    /**
     * @dev Get active raffle IDs
     */
    function getActiveRaffleIds() external view returns (uint256[] memory) {
        if (nextRaffleId <= 1) {
            return new uint256[](0);
        }
        
        uint256 count = 0;
        // First pass: count active raffles
        for (uint256 i = 1; i < nextRaffleId; i++) {
            if (raffles[i].isActive && block.timestamp < raffles[i].endTime) {
                count++;
            }
        }
        
        // Second pass: populate array
        uint256[] memory activeRaffles = new uint256[](count);
        uint256 index = 0;
        for (uint256 i = 1; i < nextRaffleId; i++) {
            if (raffles[i].isActive && block.timestamp < raffles[i].endTime) {
                activeRaffles[index] = i;
                index++;
            }
        }
        return activeRaffles;
    }

    /**
     * @dev Get ended raffle IDs
     */
    function getEndedRaffleIds() external view returns (uint256[] memory) {
        if (nextRaffleId <= 1) {
            return new uint256[](0);
        }
        
        uint256 count = 0;
        // First pass: count ended raffles
        for (uint256 i = 1; i < nextRaffleId; i++) {
            if (block.timestamp >= raffles[i].endTime) {
                count++;
            }
        }
        
        // Second pass: populate array
        uint256[] memory endedRaffles = new uint256[](count);
        uint256 index = 0;
        for (uint256 i = 1; i < nextRaffleId; i++) {
            if (block.timestamp >= raffles[i].endTime) {
                endedRaffles[index] = i;
                index++;
            }
        }
        return endedRaffles;
    }

    /**
     * @dev Get raffle IDs with prizes deposited
     */
    function getRaffleIdsWithPrizes() external view returns (uint256[] memory) {
        uint256 count = 0;
        // First pass: count raffles with prizes
        for (uint256 i = 1; i < nextRaffleId; i++) {
            if (prizes[i].isDeposited) {
                count++;
            }
        }
        
        // Second pass: populate array
        uint256[] memory rafflesWithPrizes = new uint256[](count);
        uint256 index = 0;
        for (uint256 i = 1; i < nextRaffleId; i++) {
            if (prizes[i].isDeposited) {
                rafflesWithPrizes[index] = i;
                index++;
            }
        }
        return rafflesWithPrizes;
    }

    /**
     * @dev Get raffle IDs by ticket token
     * @param _token Token address used for tickets
     */
    function getRaffleIdsByTicketToken(address _token) external view returns (uint256[] memory) {
        uint256 count = 0;
        // First pass: count raffles with this ticket token
        for (uint256 i = 1; i < nextRaffleId; i++) {
            if (raffles[i].ticketToken == _token) {
                count++;
            }
        }
        
        // Second pass: populate array
        uint256[] memory tokenRaffles = new uint256[](count);
        uint256 index = 0;
        for (uint256 i = 1; i < nextRaffleId; i++) {
            if (raffles[i].ticketToken == _token) {
                tokenRaffles[index] = i;
                index++;
            }
        }
        return tokenRaffles;
    }

    /**
     * @dev Get user's raffle participation
     * @param _user User address
     */
    function getUserRaffleParticipation(address _user) external view returns (uint256[] memory raffleIds, uint256[] memory ticketCounts) {
        uint256 count = 0;
        // First pass: count raffles where user has tickets
        for (uint256 i = 1; i < nextRaffleId; i++) {
            if (userTicketsInRaffle[i][_user] > 0) {
                count++;
            }
        }
        
        // Second pass: populate arrays
        raffleIds = new uint256[](count);
        ticketCounts = new uint256[](count);
        uint256 index = 0;
        for (uint256 i = 1; i < nextRaffleId; i++) {
            if (userTicketsInRaffle[i][_user] > 0) {
                raffleIds[index] = i;
                ticketCounts[index] = userTicketsInRaffle[i][_user];
                index++;
            }
        }
    }

    /**
     * @dev Get raffle statistics
     * @param _raffleId ID of the raffle
     */
    function getRaffleStatistics(uint256 _raffleId) external view raffleExists(_raffleId) returns (
        uint256 totalTicketsSold,
        uint256 totalRevenue,
        uint256 availableTickets,
        uint256 participationCount,
        bool hasPrize,
        bool isEnded,
        bool isFinalized
    ) {
        RaffleData memory raffle = raffles[_raffleId];
        PrizeData memory prize = prizes[_raffleId];
        
        totalTicketsSold = raffle.totalTicketsSold;
        totalRevenue = raffle.totalTicketsSold * raffle.ticketPrice;
        availableTickets = raffle.maxTickets - raffle.totalTicketsSold;
        participationCount = raffleTickets[_raffleId].length;
        hasPrize = prize.isDeposited;
        isEnded = block.timestamp >= raffle.endTime;
        isFinalized = raffle.isFinalized;
    }

    /**
     * @dev Get platform statistics
     */
    function getPlatformStatistics() external view returns (
        uint256 totalRaffles,
        uint256 activeRaffles,
        uint256 endedRaffles,
        uint256 finalizedRaffles,
        uint256 totalTicketsSold,
        uint256 totalRevenue,
        uint256 serviceChargeRate
    ) {
        if (nextRaffleId <= 1) {
            return (0, 0, 0, 0, 0, 0, platformServiceCharge);
        }
        
        totalRaffles = nextRaffleId - 1;
        serviceChargeRate = platformServiceCharge;
        
        for (uint256 i = 1; i < nextRaffleId; i++) {
            RaffleData memory raffle = raffles[i];
            
            if (raffle.isActive && block.timestamp < raffle.endTime) {
                activeRaffles++;
            }
            
            if (block.timestamp >= raffle.endTime) {
                endedRaffles++;
            }
            
            if (raffle.isFinalized) {
                finalizedRaffles++;
            }
            
            totalTicketsSold += raffle.totalTicketsSold;
            totalRevenue += raffle.totalTicketsSold * raffle.ticketPrice;
        }
    }

    /**
     * @dev Get ticket details for a raffle
     * @param _raffleId ID of the raffle
     * @param _startIndex Starting index for pagination
     * @param _count Number of tickets to return
     */
    function getRaffleTicketsPaginated(uint256 _raffleId, uint256 _startIndex, uint256 _count) external view raffleExists(_raffleId) returns (
        uint256[] memory ticketIds,
        address[] memory owners,
        uint256[] memory purchaseTimes
    ) {
        uint256[] memory allTicketIds = raffleTickets[_raffleId];
        uint256 totalTickets = allTicketIds.length;
        
        if (_startIndex >= totalTickets) {
            return (new uint256[](0), new address[](0), new uint256[](0));
        }
        
        uint256 endIndex = _startIndex + _count;
        if (endIndex > totalTickets) {
            endIndex = totalTickets;
        }
        
        uint256 resultCount = endIndex - _startIndex;
        ticketIds = new uint256[](resultCount);
        owners = new address[](resultCount);
        purchaseTimes = new uint256[](resultCount);
        
        for (uint256 i = 0; i < resultCount; i++) {
            uint256 ticketId = allTicketIds[_startIndex + i];
            ticketIds[i] = ticketId;
            owners[i] = tickets[ticketId].owner;
            purchaseTimes[i] = tickets[ticketId].purchaseTime;
        }
    }

    /**
     * @dev Get user's ticket details for a raffle
     * @param _raffleId ID of the raffle
     * @param _user User address
     */
    function getUserTicketsInRaffleDetailed(uint256 _raffleId, address _user) external view raffleExists(_raffleId) returns (
        uint256[] memory ticketIds,
        uint256[] memory purchaseTimes
    ) {
        uint256[] memory allTicketIds = raffleTickets[_raffleId];
        uint256 userTicketCount = userTicketsInRaffle[_raffleId][_user];
        
        ticketIds = new uint256[](userTicketCount);
        purchaseTimes = new uint256[](userTicketCount);
        
        uint256 index = 0;
        for (uint256 i = 0; i < allTicketIds.length && index < userTicketCount; i++) {
            uint256 ticketId = allTicketIds[i];
            if (tickets[ticketId].owner == _user) {
                ticketIds[index] = ticketId;
                purchaseTimes[index] = tickets[ticketId].purchaseTime;
                index++;
            }
        }
    }

    /**
     * @dev Get raffle winners
     */
    function getRaffleWinners() external view returns (uint256[] memory raffleIds, address[] memory winners) {
        uint256 count = 0;
        // First pass: count raffles with winners
        for (uint256 i = 1; i < nextRaffleId; i++) {
            if (raffles[i].winner != address(0)) {
                count++;
            }
        }
        
        // Second pass: populate arrays
        raffleIds = new uint256[](count);
        winners = new address[](count);
        uint256 index = 0;
        for (uint256 i = 1; i < nextRaffleId; i++) {
            if (raffles[i].winner != address(0)) {
                raffleIds[index] = i;
                winners[index] = raffles[i].winner;
                index++;
            }
        }
    }

    /**
     * @dev Get raffle by ticket ID
     * @param _ticketId ID of the ticket
     */
    function getRaffleByTicketId(uint256 _ticketId) external view returns (uint256 raffleId) {
        require(_ticketId > 0 && _ticketId < nextTicketId, "Ticket does not exist");
        return tickets[_ticketId].raffleId;
    }

    /**
     * @dev Check if user has tickets in raffle
     * @param _raffleId ID of the raffle
     * @param _user User address
     */
    function userHasTicketsInRaffle(uint256 _raffleId, address _user) external view raffleExists(_raffleId) returns (bool) {
        return userTicketsInRaffle[_raffleId][_user] > 0;
    }

    /**
     * @dev Get raffle end time
     * @param _raffleId ID of the raffle
     */
    function getRaffleEndTime(uint256 _raffleId) external view raffleExists(_raffleId) returns (uint256) {
        return raffles[_raffleId].endTime;
    }

    /**
     * @dev Get raffle creator
     * @param _raffleId ID of the raffle
     */
    function getRaffleCreator(uint256 _raffleId) external view raffleExists(_raffleId) returns (address) {
        return raffles[_raffleId].creator;
    }

    /**
     * @dev Get raffle ticket price
     * @param _raffleId ID of the raffle
     */
    function getRaffleTicketPrice(uint256 _raffleId) external view raffleExists(_raffleId) returns (uint256) {
        return raffles[_raffleId].ticketPrice;
    }

    /**
     * @dev Get raffle ticket token
     * @param _raffleId ID of the raffle
     */
    function getRaffleTicketToken(uint256 _raffleId) external view raffleExists(_raffleId) returns (address) {
        return raffles[_raffleId].ticketToken;
    }

    /**
     * @dev Get raffle max tickets
     * @param _raffleId ID of the raffle
     */
    function getRaffleMaxTickets(uint256 _raffleId) external view raffleExists(_raffleId) returns (uint256) {
        return raffles[_raffleId].maxTickets;
    }

    /**
     * @dev Get raffle description
     * @param _raffleId ID of the raffle
     */
    function getRaffleDescription(uint256 _raffleId) external view raffleExists(_raffleId) returns (string memory) {
        return raffles[_raffleId].description;
    }

    /**
     * @dev Get raffle winner
     * @param _raffleId ID of the raffle
     */
    function getRaffleWinner(uint256 _raffleId) external view raffleExists(_raffleId) returns (address) {
        return raffles[_raffleId].winner;
    }

    /**
     * @dev Get raffle winning ticket ID
     * @param _raffleId ID of the raffle
     */
    function getRaffleWinningTicketId(uint256 _raffleId) external view raffleExists(_raffleId) returns (uint256) {
        return raffles[_raffleId].winningTicketId;
    }

    /**
     * @dev Check if raffle is finalized
     * @param _raffleId ID of the raffle
     */
    function isRaffleFinalized(uint256 _raffleId) external view raffleExists(_raffleId) returns (bool) {
        return raffles[_raffleId].isFinalized;
    }

    /**
     * @dev Get next raffle ID
     */
    function getNextRaffleId() external view returns (uint256) {
        return nextRaffleId;
    }

    /**
     * @dev Get next ticket ID
     */
    function getNextTicketId() external view returns (uint256) {
        return nextTicketId;
    }

    /**
     * @dev Get platform owner
     */
    function getPlatformOwner() external view returns (address) {
        return platformOwner;
    }

    // ============ VRF FUNCTIONS ============

    /**
     * @dev Configure VRF parameters (only platform owner)
     * @param _subscriptionId VRF subscription ID
     * @param _keyHash VRF key hash
     * @param _callbackGasLimit Gas limit for VRF callback
     * @param _requestConfirmations Number of confirmations for VRF request
     */
    function configureVRF(
        uint256 _subscriptionId,
        bytes32 _keyHash,
        uint32 _callbackGasLimit,
        uint16 _requestConfirmations
    ) external onlyPlatformOwner {
        s_subscriptionId = _subscriptionId;
        s_keyHash = _keyHash;
        s_callbackGasLimit = _callbackGasLimit;
        s_requestConfirmations = _requestConfirmations;

        emit VRFConfigurationUpdated(_subscriptionId, _keyHash, _callbackGasLimit, _requestConfirmations);
    }

    /**
     * @dev Get VRF configuration
     */
    function getVRFConfiguration() external view returns (
        uint256 subscriptionId,
        bytes32 keyHash,
        uint32 callbackGasLimit,
        uint16 requestConfirmations,
        uint32 numWords
    ) {
        return (s_subscriptionId, s_keyHash, s_callbackGasLimit, s_requestConfirmations, s_numWords);
    }

    /**
     * @dev Check if a raffle has a pending VRF request
     * @param _raffleId ID of the raffle
     */
    function hasPendingVRFRequest(uint256 _raffleId) external view raffleExists(_raffleId) returns (bool) {
        return pendingVRFRequests[_raffleId];
    }

    /**
     * @dev Receive function to accept ETH payments
     */
    receive() external payable {
        // Accept ETH payments for VRF requests
    }
}
