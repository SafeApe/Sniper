// SPDX-License-Identifier: Unlicensed
pragma solidity ^0.8.6;
//Abstract Contracts
import {PeripheryPayments} from "./PaymentHelperUniV3.sol";

// interfaces
import "./interfaces/IRouterV2.sol";
import "./interfaces/IFactory.sol";
import "./interfaces/IPairV2.sol";
import "./interfaces/IERC20.sol";
import "./interfaces/ICHI.sol"; //chi gas token
import {IUniswapV3Pool} from "./interfaces/IUniswapV3Pool.sol";
import {ISwapRouter02} from "./interfaces/ISwapRouterV2.sol";

// libraries
import "./libraries/TickMath.sol";
import "./libraries/SafeCast.sol";
import "./libraries/path.sol";
// import {UniswapV2Library} from './libraries/UniswapV2Library.sol';

struct SwapCallbackData {
    bytes path;
    address payer;
}

struct ExactInputSingleParams {
    address tokenIn;
    address tokenOut;
    uint24 fee;
    address recipient;
    uint256 amountIn;
    uint256 amountOutMinimum;
    uint160 sqrtPriceLimitX96;
}

library TransferHelper {
    function safeApprove(address token, address to, uint value) internal {
        // bytes4(keccak256(bytes('approve(address,uint256)')));
        (bool success, bytes memory data) = token.call(
            abi.encodeWithSelector(0x095ea7b3, to, value)
        );
        require(
            success && (data.length == 0 || abi.decode(data, (bool))),
            "TransferHelper: APPROVE_FAILED"
        );
    }

    function safeTransfer(address token, address to, uint value) internal {
        // bytes4(keccak256(bytes('transfer(address,uint256)')));
        (bool success, bytes memory data) = token.call(
            abi.encodeWithSelector(0xa9059cbb, to, value)
        );
        require(
            success && (data.length == 0 || abi.decode(data, (bool))),
            "TransferHelper: TRANSFER_FAILED"
        );
    }

    function safeTransferFrom(
        address token,
        address from,
        address to,
        uint value
    ) internal {
        // bytes4(keccak256(bytes('transferFrom(address,address,uint256)')));
        (bool success, bytes memory data) = token.call(
            abi.encodeWithSelector(0x23b872dd, from, to, value)
        );
        require(
            success && (data.length == 0 || abi.decode(data, (bool))),
            "TransferHelper: TRANSFER_FROM_FAILED"
        );
    }

    function safeTransferETH(address to, uint value) internal {
        (bool success, ) = to.call{value: value}(new bytes(0));
        require(success, "TransferHelper: ETH_TRANSFER_FAILED");
    }
}

library SafeMath {
    function add(uint x, uint y) internal pure returns (uint z) {
        require((z = x + y) >= x, "ds-math-add-overflow");
    }

    function sub(uint x, uint y) internal pure returns (uint z) {
        require((z = x - y) <= x, "ds-math-sub-underflow");
    }

    function mul(uint x, uint y) internal pure returns (uint z) {
        require(y == 0 || (z = x * y) / y == x, "ds-math-mul-overflow");
    }
}

library SwapHelper {
    using SafeMath for uint;

    function sortTokens(
        address tokenA,
        address tokenB
    ) internal pure returns (address token0, address token1) {
        require(tokenA != tokenB, " Error: IDENTICAL_ADDRESSES");
        (token0, token1) = tokenA < tokenB
            ? (tokenA, tokenB)
            : (tokenB, tokenA);
        require(token0 != address(0), "Error: ZERO_ADDRESS");
    }

    function getAmountOut(
        uint amountIn,
        uint reserveIn,
        uint reserveOut
    ) internal pure returns (uint amountOut) {
        require(amountIn > 0, "UniswapV2Library: INSUFFICIENT_INPUT_AMOUNT");
        require(
            reserveIn > 0 && reserveOut > 0,
            "UniswapV2Library: INSUFFICIENT_LIQUIDITY"
        );
        uint amountInWithFee = amountIn.mul(997);
        uint numerator = amountInWithFee.mul(reserveOut);
        uint denominator = reserveIn.mul(1000).add(amountInWithFee);
        amountOut = numerator / denominator;
    }

    function getReserves(
        ILiquidityPairV2 _pair,
        address tokenA,
        address tokenB
    ) internal view returns (uint reserveA, uint reserveB) {
        (address token0, ) = sortTokens(tokenA, tokenB);
        (uint reserve0, uint reserve1, ) = _pair.getReserves();
        (reserveA, reserveB) = tokenA == token0
            ? (reserve0, reserve1)
            : (reserve1, reserve0);
    }

    function getAmountsOut(
        IFactory _factory,
        uint amountIn,
        address[] memory path
    ) internal view returns (uint[] memory amounts) {
        require(path.length >= 2, "Error: INVALID_PATH");
        amounts = new uint[](path.length);
        amounts[0] = amountIn;
        ILiquidityPairV2 _pair = ILiquidityPairV2(
            _factory.getPair(path[0], path[1])
        );
        for (uint i; i < path.length - 1; i++) {
            (uint reserveIn, uint reserveOut) = getReserves(
                _pair,
                path[i],
                path[i + 1]
            );
            amounts[i + 1] = getAmountOut(amounts[i], reserveIn, reserveOut);
        }
    }
}

contract SniperSwapper is PeripheryPayments {
    using SafeMath for uint;
    using SafeCast for uint256;
    using Path for bytes;
    address public owner;
    uint public chainid;
    ChiGasToken private chi;
    address public WETH;

    address _temp_real_payer;
    address tempCallbackCheck;

    struct ExchangeData {
        address factory;
        string init_code;
    }
    mapping(address => ExchangeData) exchanges;

    function setChainData(uint _chainid) internal {
        if (_chainid == 1) {
            // Setting in Default Required values
            WETH = 0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2;
            chi = ChiGasToken(0x0000000000004946c0e9F43F4Dee607b0eF1fA1c);
        } else if (_chainid == 5) {
            WETH = 0xB4FBF271143F4FBf7B91A5ded31805e42b2208d6;
            chi = ChiGasToken(0x0000000000004946c0e9F43F4Dee607b0eF1fA1c);
        } else if (_chainid == 56) {
            WETH = 0xbb4CdB9CBd36B01bD1cBaEBF2De08d9173bc095c;
            chi = ChiGasToken(0x0000000000004946c0e9F43F4Dee607b0eF1fA1c);
        } else if (_chainid == 137) {
            WETH = 0x0d500B1d8E8eF31E21C99d1Db9A6444d3ADf1270;
            chi = ChiGasToken(0x0000000000004946c0e9F43F4Dee607b0eF1fA1c);
        } else if (_chainid == 43114 || _chainid == 31337) {
            WETH = 0x4200000000000000000000000000000000000006; //BASE FALLBACK on test network
            chi = ChiGasToken(address(0x0));
        } else if (_chainid == 8453) {
            WETH = 0x4200000000000000000000000000000000000006;
            chi = ChiGasToken(address(0x0));
        } else {
            WETH = address(0x0);
            chi = ChiGasToken(address(0x0));
        }
    }

    constructor() PeripheryPayments() {
        owner = msg.sender;
        chainid = block.chainid;
        setChainData(chainid);
        setFactoryandWETH9(address(0x0), WETH);
    }

    modifier discountCHI() {
        if (address(chi) != address(0x0)) {
            uint256 gasStart = gasleft();

            _;

            uint256 initialGas = 21000 + 16 * msg.data.length;
            uint256 gasSpent = initialGas + gasStart - gasleft();
            uint256 freeUpValue = (gasSpent + 14154) / 41947;

            chi.freeFromUpTo(msg.sender, freeUpValue);
        }
    }

    modifier onlyOwner() {
        require(owner == msg.sender, "Not Owner");
        _;
    }

    function transferOwnership(address newOwner) public onlyOwner {
        owner = newOwner;
    }

    function RouterSwapBySplitV2(
        address _router,
        address _tokenIn,
        address _tokenOut,
        uint _amountOutMin,
        address _to,
        uint _split
    ) external payable discountCHI {
        address[] memory path;
        path = new address[](2);
        path[0] = _tokenIn;
        path[1] = _tokenOut;
        uint splittedPayment = msg.value / _split;
        for (uint i = 0; i < _split; i++) {
            IRouterV2(_router).swapExactETHForTokens{value: splittedPayment}(
                _amountOutMin,
                path,
                _to,
                block.timestamp
            );
        }
    }

    function TradeDirectlyByPairV2(
        address _pair,
        address _tokenIn,
        uint _amountin
    ) public payable {
        address receipent = msg.sender;
        _temp_real_payer = receipent;
        ILiquidityPairV2 pair = ILiquidityPairV2(_pair);
        address token0 = pair.token0();
        address token1 = pair.token1();
        address _tokenOut = token0 == _tokenIn ? token1 : token0;
        {
            if (_tokenIn == WETH && msg.value > 0) {
                IERC20(WETH).deposit{value: msg.value}();
                TransferHelper.safeApprove(WETH, _pair, _amountin);
                _temp_real_payer = address(this);
            } else if (_tokenOut == WETH) receipent = address(this);
            pay(_tokenIn, _temp_real_payer, _pair, _amountin);
        }
        uint amountOut;
        {
            (uint reserve0, uint reserve1) = SwapHelper.getReserves(
                pair,
                token0,
                token1
            );
            (uint reservein, uint reserveout) = token0 == _tokenIn
                ? (reserve0, reserve1)
                : (reserve1, reserve0);
            amountOut = SwapHelper.getAmountOut(
                IERC20(_tokenIn).balanceOf(address(pair)).sub(reservein),
                reservein,
                reserveout
            );
        }
        if (token0 == _tokenIn) {
            pair.swap(0, amountOut, receipent, new bytes(0));
        } else {
            pair.swap(amountOut, 0, receipent, new bytes(0));
        }
        if (_tokenOut == WETH) {
            uint256 amountout = IERC20(WETH).balanceOf(address(this));
            IERC20(WETH).withdraw(amountout);
            TransferHelper.safeTransferETH(msg.sender, amountout);
        }
    }

    /// To Support Uniswap V3 Pool Swap Callbacks
    function uniswapV3SwapCallback(
        int256 amount0Delta,
        int256 amount1Delta,
        bytes calldata _data
    ) external {
        require(amount0Delta > 0 || amount1Delta > 0); // swaps entirely within 0-liquidity regions are not supported
        SwapCallbackData memory data = abi.decode(_data, (SwapCallbackData));
        (address tokenIn, address tokenOut, uint24 fee) = data
            .path
            .decodeFirstPool();
        // CallbackValidation.verifyCallback(factory, tokenIn, tokenOut, fee);
        require(
            msg.sender == tempCallbackCheck,
            "UniswapV3SwapCallback: Invalid sender"
        );

        (bool isExactInput, uint256 amountToPay) = amount0Delta > 0
            ? (tokenIn < tokenOut, uint256(amount0Delta))
            : (tokenOut < tokenIn, uint256(amount1Delta));
        pay(tokenIn, _temp_real_payer, msg.sender, amountToPay);
        // else {
        //     // either initiate the next swap or pay
        //     if (data.path.hasMultiplePools()) {
        //         data.path = data.path.skipToken();
        //         exactOutputInternal(amountToPay, msg.sender, 0, data);
        //     } else {
        //         amountInCached = amountToPay;
        //         // note that because exact output swaps are executed in reverse order, tokenOut is actually tokenIn
        //         pay(tokenOut, data.payer, msg.sender, amountToPay);
        //     }
        // }
    }

    function TradeDirectlyByPairV3(
        address _pair,
        address _tokenin,
        uint256 _amountin
    ) public payable {
        address receipent = msg.sender;
        _temp_real_payer = receipent;
        IUniswapV3Pool pool = IUniswapV3Pool(_pair);
        tempCallbackCheck = _pair;
        uint24 fee = pool.fee();
        uint sqrtPriceLimitX96 = 0; // Considered as 0 for now
        address _token0 = pool.token0();
        address _token1 = pool.token1();
        address _tokenout = _token0 == _tokenin ? _token1 : _token0;
        bool zeroForOne = _tokenin < _tokenout;
        SwapCallbackData memory data = SwapCallbackData({
            path: abi.encodePacked(_tokenin, fee, _tokenout),
            payer: address(this)
        });
        if (_tokenout == WETH) {
            _temp_real_payer = receipent;
            receipent = address(this);
        }
        pool.swap(
            receipent,
            zeroForOne,
            _amountin.toInt256(),
            uint160(
                sqrtPriceLimitX96 == 0
                    ? (
                        zeroForOne
                            ? TickMath.MIN_SQRT_RATIO + 1
                            : TickMath.MAX_SQRT_RATIO - 1
                    )
                    : sqrtPriceLimitX96
            ),
            abi.encode(data)
        );
        if (receipent == address(this) && _tokenout == WETH) {
            uint256 amountout = IERC20(WETH).balanceOf(address(this));
            IERC20(WETH).withdraw(amountout);
            TransferHelper.safeTransferETH(msg.sender, amountout);
        }
        delete _temp_real_payer;
    }

    function HoneyPotCheck(
        address _router,
        address[] memory path,
        uint _split
    ) external payable returns (uint, uint, uint, uint) {
        uint tokens_to_get = IRouterV2(_router).getAmountsOut(msg.value, path)[
            1
        ];
        address token = path[1];
        address _weth = path[0];
        for (uint i; i < _split; i++) {
            IRouterV2(_router)
                .swapExactETHForTokensSupportingFeeOnTransferTokens{
                value: msg.value / _split
            }(0, path, address(this), block.timestamp + 15);
        }
        uint tokens_got = IERC20(token).balanceOf(address(this));
        IERC20(token).approve(address(_router), tokens_got);
        path[0] = token;
        path[1] = _weth;
        uint eth_to_get = IRouterV2(_router).getAmountsOut(tokens_got, path)[1];
        for (uint i; i < _split; i++) {
            IRouterV2(_router)
                .swapExactTokensForETHSupportingFeeOnTransferTokens(
                    tokens_got / _split,
                    0,
                    path,
                    address(this),
                    block.timestamp + 15
                );
        }
        uint eth_got = address(this).balance;
        return (tokens_to_get, tokens_got, eth_to_get, eth_got);
    }

    function swap(
        address _pair,
        address _tokenIn,
        uint _amountin
    ) public payable {
        ILiquidityPairV2 pair = ILiquidityPairV2(_pair);
        (bool success, bytes memory data) = address(pair).staticcall(
            abi.encodeWithSignature("getReserves()")
        );
        if (success) {
            TradeDirectlyByPairV2(_pair, _tokenIn, _amountin);
            return;
        }
        TradeDirectlyByPairV3(_pair, _tokenIn, _amountin);
    }

    // function CheckHPWithPair(address[] memory _pairsToHop,address[] memory _tokenIns, uint memory amountin) payable public {
    //     uint amountOut = amountIn;
    //     for(uint i;i<_pairsToHop.length;i++){
    //         ILiquidityPairV2 _pair = ILiquidityPairV2(_pairsToHop[i]);
    //         address _tokenIn = _tokenIns[i];
    //         address _tokenOut = _pair.token0() == _tokenIn ? _pair.token1() : _pair.token0();
    //         uint amountOut;
    //         swap(_pair,_tokenIn,amountOut);
    //         amountIn
    //     }
    // }

    function checkContractAllowance(
        address _token,
        uint _amount
    ) public view returns (bool) {
        uint allowance = IERC20(_token).allowance(msg.sender, address(this));
        return allowance >= _amount;
    }

    function recoverERC20orETH(address _token, uint _amount) public onlyOwner {
        if (_token == address(0)) {
            TransferHelper.safeTransferETH(owner, _amount);
        } else {
            TransferHelper.safeTransfer(_token, owner, _amount);
        }
    }

    fallback() external payable {}
}
