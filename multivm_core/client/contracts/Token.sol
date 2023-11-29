// SPDX-License-Identifier: MIT
pragma solidity ^0.8.18;

import "@openzeppelin/contracts/token/ERC20/extensions/ERC20Burnable.sol";
import "@openzeppelin/contracts/token/ERC20/ERC20.sol";


contract Token is ERC20, ERC20Burnable {
    constructor(string memory name_, string memory symbol_, uint totalSupply_) ERC20(name_, symbol_) {
        _mint(msg.sender, totalSupply_);
    }
}