//SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.7.0;

contract TestSafeMath {
    uint256 a;

    using SafeMath for uint256;

    constructor() {
        a = 0xAAAA;
    }

    function sub() public {
        a = a.sub(1);
    }

    function get() public view returns (uint256) {
        return a;
    }
}

library SafeMath {
    function sub(uint256 a, uint256 b) internal pure returns (uint256) {
        assert(b <= a);
        return a - b;
    }

    function add(uint256 a, uint256 b) internal pure returns (uint256) {
        uint256 c = a + b;
        assert(c >= a);
        return c;
    }
}
