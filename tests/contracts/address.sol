// SPDX-License-Identifier: GPL-3.0
pragma solidity >=0.4.16 <0.8.0;

contract SimpleAddress {
    address storedData;

    constructor() {
        storedData = msg.sender;
    }

    function set(address x) public {
        storedData = x;
    }

    function get() public view returns (address) {
        return storedData;
    }
}