// This file is part of Substrate.

// Copyright (C) 2019-2022 Parity Technologies (UK) Ltd.
// Some code is modified from Derek Dreery's IntegerSquareRoot impl.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Some helper functions to work with 128bit numbers. Note that the functionality provided here is
//! only sensible to use with 128bit numbers because for smaller sizes, you can always rely on
//! assumptions of a bigger type (u128) being available, or simply create a per-thing and use the
//! multiplication implementation provided there.

use crate::{biguint, Rounding};
use sp_std::convert::TryInto;
use num_traits::Zero;
use sp_std::{
	cmp::{max, min},
	mem,
};

/// Helper gcd function used in Rational128 implementation.
pub fn gcd(a: u128, b: u128) -> u128 {
	match ((a, b), (a & 1, b & 1)) {
		((x, y), _) if x == y => y,
		((0, x), _) | ((x, 0), _) => x,
		((x, y), (0, 1)) | ((y, x), (1, 0)) => gcd(x >> 1, y),
		((x, y), (0, 0)) => gcd(x >> 1, y >> 1) << 1,
		((x, y), (1, 1)) => {
			let (x, y) = (min(x, y), max(x, y));
			gcd((y - x) >> 1, x)
		},
		_ => unreachable!(),
	}
}

/// split a u128 into two u64 limbs
pub fn split(a: u128) -> (u64, u64) {
	let al = a as u64;
	let ah = (a >> 64) as u64;
	(ah, al)
}

/// Convert a u128 to a u32 based biguint.
pub fn to_big_uint(x: u128) -> biguint::BigUint {
	let (xh, xl) = split(x);
	let (xhh, xhl) = biguint::split(xh);
	let (xlh, xll) = biguint::split(xl);
	let mut n = biguint::BigUint::from_limbs(&[xhh, xhl, xlh, xll]);
	n.lstrip();
	n
}

/// Safely and accurately compute `a * b / c`. The approach is:
///   - Simply try `a * b / c`.
///   - Else, convert them both into big numbers and re-try. `Err` is returned if the result cannot
///     be safely casted back to u128.
///
/// Invariant: c must be greater than or equal to 1.
pub fn multiply_by_rational(mut a: u128, mut b: u128, mut c: u128) -> Result<u128, &'static str> {
	if a.is_zero() || b.is_zero() {
		return Ok(Zero::zero())
	}
	c = c.max(1);

	// a and b are interchangeable by definition in this function. It always helps to assume the
	// bigger of which is being multiplied by a `0 < b/c < 1`. Hence, a should be the bigger and
	// b the smaller one.
	if b > a {
		mem::swap(&mut a, &mut b);
	}

	// Attempt to perform the division first
	if a % c == 0 {
		a /= c;
		c = 1;
	} else if b % c == 0 {
		b /= c;
		c = 1;
	}

	if let Some(x) = a.checked_mul(b) {
		// This is the safest way to go. Try it.
		Ok(x / c)
	} else {
		let a_num = to_big_uint(a);
		let b_num = to_big_uint(b);
		let c_num = to_big_uint(c);

		let mut ab = a_num * b_num;
		ab.lstrip();
		let mut q = if c_num.len() == 1 {
			// PROOF: if `c_num.len() == 1` then `c` fits in one limb.
			ab.div_unit(c as biguint::Single)
		} else {
			// PROOF: both `ab` and `c` cannot have leading zero limbs; if length of `c` is 1,
			// the previous branch would handle. Also, if ab for sure has a bigger size than
			// c, because `a.checked_mul(b)` has failed, hence ab must be at least one limb
			// bigger than c. In this case, returning zero is defensive-only and div should
			// always return Some.
			let (mut q, r) = ab.div(&c_num, true).unwrap_or((Zero::zero(), Zero::zero()));
			let r: u128 = r.try_into().expect("reminder of div by c is always less than c; qed");
			if r > (c / 2) {
				q = q.add(&to_big_uint(1));
			}
			q
		};
		q.lstrip();
		q.try_into().map_err(|_| "result cannot fit in u128")
	}
}

mod double128 {
	// Inspired by: https://medium.com/wicketh/mathemagic-512-bit-division-in-solidity-afa55870a65
	use num_traits::Zero;
	use sp_std::convert::TryFrom;

	/// Returns the least significant 64 bits of a
	const fn low_64(a: u128) -> u128 {
		a & ((1<<64)-1)
	}

	/// Returns the most significant 64 bits of a
	const fn high_64(a: u128) -> u128 {
		a >> 64
	}

	/// Returns 2^128 - a (two's complement)
	const fn neg128(a: u128) -> u128 {
		(!a).wrapping_add(1)
	}

	/// Returns 2^128 / a
	const fn div128(a: u128) -> u128 {
		(neg128(a)/a).wrapping_add(1)
	}

	/// Returns 2^128 % a
	const fn mod128(a: u128) -> u128 {
		neg128(a) % a
	}

	#[derive(Copy, Clone, Eq, PartialEq)]
	pub struct Double128 {
		high: u128,
		low: u128,
	}

	impl TryFrom<Double128> for u128 {
		type Error = ();
		fn try_from(x: Double128) -> Result<Self, ()> {
			x.try_into_u128()
		}
	}

	impl Zero for Double128 {
		fn zero() -> Self {
			Double128::zero()
		}
		fn is_zero(&self) -> bool {
			Double128::is_zero(&self)
		}
	}

	impl sp_std::ops::Add<Self> for Double128 {
		type Output = Self;
		fn add(self, rhs: Self) -> Self {
			Double128::add(self, rhs)
		}
	}

	impl sp_std::ops::AddAssign<Self> for Double128 {
		fn add_assign(&mut self, rhs: Self) {
			*self = self.add(rhs);
		}
	}

	impl sp_std::ops::Div<u128> for Double128 {
		type Output = (Self, u128);
		fn div(self, rhs: u128) -> (Self, u128) {
			Double128::div(self, rhs)
		}
	}

	impl Double128 {
		pub const fn try_into_u128(self) -> Result<u128, ()> {
			match self.high {
				0 => Ok(self.low),
				_ => Err(()),
			}
		}

		pub const fn zero() -> Self {
			Self {
				high: 0,
				low: 0,
			}
		}

		pub const fn is_zero(&self) -> bool {
			self.high == 0 && self.low == 0
		}

		/// Return a `Double128` value representing the `scaled_value << 64`.
		///
		/// This means the lower half of the `high` component will be equal to the upper 64-bits of
		/// `scaled_value` (in the lower positions) and the upper half of the `low` component will
		/// be equal to the lower 64-bits of `scaled_value`.
		pub const fn left_shift_64(scaled_value: u128) -> Self {
			Self {
				high: scaled_value >> 64,
				low: scaled_value << 64,
			}
		}

		/// Construct a value from the upper 128 bits only, with the lower being zeroed.
		pub const fn from_low(low: u128) -> Self {
			Self { high: 0, low }
		}

		/// Returns the same value ignoring anything in the high 128-bits.
		pub const fn low_part(self) -> Self {
			Self { high: 0, .. self }
		}

		/// Returns a*b (in 256 bits)
		pub const fn product_of(a: u128, b: u128) -> Self {
			// Split a and b into hi and lo 64-bit parts
			let (a_low, a_high) = (low_64(a), high_64(a));
			let (b_low, b_high) = (low_64(b), high_64(b));
			// a = (a_low + a_high << 64); b = (b_low + b_high << 64);
			// ergo a*b = (a_low + a_high << 64)(b_low + b_high << 64)
			//          = a_low * b_low
			//          + a_low * b_high << 64
			//          + a_high << 64 * b_low
			//          + a_high << 64 * b_high << 64
			// assuming:
			//        f = a_low * b_low
			//        o = a_low * b_high
			//        i = a_high * b_low
			//        l = a_high * b_high
			// then:
			//      a*b = (o+i) << 64 + f + l << 128
			let (f, o, i, l) = (a_low * b_low, a_low * b_high, a_high * b_low, a_high * b_high);
			let fl = Self { high: l, low: f };
			let i = Self::left_shift_64(i);
			let o = Self::left_shift_64(o);
			fl.add(i).add(o)
		}

		pub const fn add(self, b: Self) -> Self {
			let (low, overflow) = self.low.overflowing_add(b.low);
			let carry = overflow as u128;		// 1 if true, 0 if false.
			let high = self.high.wrapping_add(b.high).wrapping_add(carry as u128);
			Double128 { high, low }
		}

		pub const fn div(mut self, rhs: u128) -> (Self, u128) {
			if rhs == 1 {
				return (self, 0);
			}

			// (self === a; rhs === b)
			// Calculate a / b
			// = (a_high << 128 + a_low) / b
			//   let (q, r) = (div128(b), mod128(b));
			// = (a_low * (q * b + r)) + a_high) / b
			// = (a_low * q * b + a_low * r + a_high)/b
			// = (a_low * r + a_high) / b + a_low * q
			let (q, r) = (div128(rhs), mod128(rhs));

			// x = current result
			// a = next number
			let mut x = Self::zero();
			while self.high != 0 {
				// x += a.low * q
				x = x.add(Self::product_of(self.high, q));
				// a = a.low * r + a.high
				self = Self::product_of(self.high, r).add(self.low_part());
			}

			(x.add(Self::from_low(self.low / rhs)), self.low % rhs)
		}
	}
}

pub const fn checked_mul(a: u128, b: u128) -> Option<u128> {
	a.checked_mul(b)
}

pub const fn checked_neg(a: u128) -> Option<u128> {
	a.checked_neg()
}

pub const fn saturating_add(a: u128, b: u128) -> u128 {
	a.saturating_add(b)
}

pub const fn sqrt(mut n: u128) -> u128 {
	// Modified from https://github.com/derekdreery/integer-sqrt-rs (Apache/MIT).
	if n == 0 { return 0 }

	// Compute bit, the largest power of 4 <= n
	let max_shift: u32 = 0u128.leading_zeros() - 1;
	let shift: u32 = (max_shift - n.leading_zeros()) & !1;
	let mut bit = 1u128 << shift;

	// Algorithm based on the implementation in:
	// https://en.wikipedia.org/wiki/Methods_of_computing_square_roots#Binary_numeral_system_(base_2)
	// Note that result/bit are logically unsigned (even if T is signed).
	let mut result = 0u128;
	while bit != 0 {
		if n >= result + bit {
			n -= result + bit;
			result = (result >> 1) + bit;
		} else {
			result = result >> 1;
		}
		bit = bit >> 2;
	}
	result
}

/// Returns `a * b / c` and `(a * b) % c` (wrapping to 128 bits) or `None` in the case of
/// overflow.
pub const fn multiply_by_rational_with_rounding(a: u128, b: u128, c: u128, r: Rounding) -> Option<u128> {
	use double128::Double128;
	if c == 0 {
		panic!("attempt to divide by zero")
	}
	let (result, remainder) = Double128::product_of(a, b).div(c);
	let mut result: u128 = match result.try_into_u128() { Ok(v) => v, Err(_) => return None };
	if match r {
		Rounding::Up => remainder > 0,
		Rounding::Nearest => remainder >= c / 2 + c % 2,
		Rounding::Down => false,
	} {
		result = match result.checked_add(1) { Some(v) => v, None => return None };
	}
	Some(result)
}

#[cfg(test)]
mod tests {
	use super::*;
	use Rounding::*;
	use multiply_by_rational_with_rounding as mulrat;
	use codec::{Encode, Decode};

	const MAX: u128 = u128::max_value();

	#[test]
	fn rational_multiply_basic_rounding_works() {
		assert_eq!(mulrat(1, 1, 1, Up), Some(1));
		assert_eq!(mulrat(3, 1, 3, Up), Some(1));
		assert_eq!(mulrat(1, 2, 3, Down), Some(0));
		assert_eq!(mulrat(1, 1, 3, Up), Some(1));
		assert_eq!(mulrat(1, 2, 3, Nearest), Some(1));
		assert_eq!(mulrat(1, 1, 3, Nearest), Some(0));
	}

	#[test]
	fn rational_multiply_big_number_works() {
		assert_eq!(mulrat(MAX, MAX-1, MAX, Down), Some(MAX-1));
		assert_eq!(mulrat(MAX, 1, MAX, Down), Some(1));
		assert_eq!(mulrat(MAX, MAX-1, MAX, Up), Some(MAX-1));
		assert_eq!(mulrat(MAX, 1, MAX, Up), Some(1));
		assert_eq!(mulrat(1, MAX-1, MAX, Down), Some(0));
		assert_eq!(mulrat(1, 1, MAX, Up), Some(1));
		assert_eq!(mulrat(1, MAX/2, MAX, Nearest), Some(0));
		assert_eq!(mulrat(1, MAX/2+1, MAX, Nearest), Some(1));
	}

	fn random_u128(seed: u32) -> u128 {
		u128::decode(&mut &seed.using_encoded(sp_core::hashing::twox_128)[..]).unwrap_or(0)
	}

	#[test]
	fn op_checked_rounded_div_works() {
		for i in 0..100_000u32 {
			let a = random_u128(i);
			let b = random_u128(i + 1 << 30);
			let c = random_u128(i + 1 << 31);
			let x = mulrat(a, b, c, Nearest);
			let y = multiply_by_rational(a, b, c).ok();
			assert_eq!(x.is_some(), y.is_some());
			let x = x.unwrap_or(0);
			let y = y.unwrap_or(0);
			let d = x.max(y) - x.min(y);
			assert_eq!(d, 0);
		}
	}
}