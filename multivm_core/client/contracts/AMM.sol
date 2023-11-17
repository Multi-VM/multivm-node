// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

import "@openzeppelin/contracts/token/ERC20/extensions/ERC20Burnable.sol";
import "@openzeppelin/contracts/token/ERC20/ERC20.sol";


contract AMM {
    // multivm-evm abi compability lifehack
    function init(string memory input) public returns (string memory) {
        return input;
    }
}