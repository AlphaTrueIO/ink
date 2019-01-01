//! Utilities to operate on byte or slices of bytes.

use core::mem::size_of;

/// Add byte to the given byte slice.
///
/// Returns `true` if there was an overflow.
///
/// # Note
///
/// For this the byte slices are interpreted as twos-complement numbers.
///
/// # Panics
///
/// If `lhs` is an empty slice.
fn bytes_add_byte(lhs: &mut [u8], rhs: u8) -> bool {
	assert!(lhs.len() >= 1);
	let mut carry = rhs;
	for lhs in lhs.into_iter().rev() {
		if carry == 0 {
			return false
		}
		let (res, ovfl) = lhs.overflowing_add(carry);
		*lhs = res;
		carry = u8::from(ovfl);
	}
	if carry == 0 { false } else { true }
}

/// Flips all bytes in the byte slice inplace.
fn invert_bytes(bytes: &mut [u8]) {
	for byte in bytes.into_iter() {
		*byte = !*byte;
	}
}

/// Negate the given bytes inplace.
///
/// Interprets the bytes as twos-complement number.
pub fn negate_bytes(bytes: &mut [u8]) {
	invert_bytes(bytes);
	bytes_add_byte(bytes, 0x01);
}

macro_rules! impl_slice_as_array {
	( $name:ident, $n:expr ) => {
		/// Interprets the slice as exact size array if possible.
		///
		/// Otherwise returns `None`.
		pub fn $name<T>(slice: &[T]) -> Option<&[T; $n]> {
			if slice.len() != $n {
				return None
			}
			Some(unsafe {
				core::mem::transmute::<*const T, &[T; $n]>(slice.as_ptr())
			})
		}
	};
}

impl_slice_as_array!(slice4_as_array4, 4);
impl_slice_as_array!(slice8_as_array8, 8);

/// Adds the given bytes slices inplace.
///
/// For this the byte slices are interpreted as twos-complement numbers.
pub fn bytes_add_bytes(lhs: &mut [u8], rhs: &[u8]) {
	assert_eq!(lhs.len(), rhs.len());
	let mut carry = 0;
	for (lhs, rhs) in lhs.into_iter().zip(rhs.into_iter()).rev() {
		let (res1, carry1) = lhs.overflowing_add(carry);
		let (res2, carry2) = res1.overflowing_add(*rhs);
		debug_assert!(!(carry1 && carry2));
		*lhs = res2;
		carry = u8::from(carry1 || carry2);
	}
}

macro_rules! primitives_impl {
	( $prim:ty, $bytes_to_prim:ident, $prim_to_bytes:ident ) => {
		/// Converts the byte array to the primitive number.
		///
		/// # Panics
		///
		/// If the byte slice does not match the number of byte
		/// in the primitive.
		pub fn $bytes_to_prim(bytes: &[u8; size_of::<$prim>()]) -> $prim {
			let mut res = 0;
			const N_BYTES: usize = size_of::<$prim>();
			const N_BITS: usize = N_BYTES * 8;
			for i in 0..N_BYTES {
				res |= (bytes[i] as $prim) << (N_BITS - ((i + 1) * 8));
			}
			res
		}

		/// Converts the primitive number to a byte array.
		pub fn $prim_to_bytes(val: $prim) -> [u8; size_of::<$prim>()] {
			const N_BYTES: usize = size_of::<$prim>();
			const N_BITS: usize = N_BYTES * 8;
			let mut buf = [0x0; N_BYTES];
			for i in 0..N_BYTES {
				buf[i] = ((val >> (N_BITS - ((i + 1) * 8))) & 0xFF) as u8
			}
			buf
		}
	};
}

primitives_impl!(u32, bytes4_to_u32, u32_to_bytes4);
primitives_impl!(u64, bytes8_to_u64, u64_to_bytes8);

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_negate_bytes() {
		fn negate_bytes_copy(bytes: &[u8]) -> Vec<u8> {
			let mut buf = bytes.to_vec();
			negate_bytes(&mut buf);
			buf
		}
		fn test_for(bytes: &[u8], expected: &[u8]) {
			assert_eq!(negate_bytes_copy(bytes), expected)
		}
		// Test for '0'
		test_for(&[0x00], &[0x00]);
		// Test for '1'
		test_for(&[0x00, 0x01], &[0xFF, 0xFF]);
		// Test for '-1' == '0xFF'
		test_for(&[0xFF, 0xFF], &[0x00, 0x01]);
		// Test for '42' == '0x2A'
		//
		// 0000 0000 0010 1010 | input
		// 1111 1111 1101 0101 | flipped
		// 1111 1111 1101 0110 | +1
		//    F    F    D    6 | hex
		test_for(&[0x00, 0x2A], &[0xFF, 0xD6]);

	}

	#[test]
	fn test_slice_as_array() {
		assert_eq!(slice4_as_array4::<i32>(&[]), None);
		assert_eq!(slice4_as_array4(&[1, 2, 3, 4, 5]), None);
		assert_eq!(slice4_as_array4(&[1, 2, 3, 4]), Some(&[1, 2, 3, 4]));
		assert_eq!(slice4_as_array4(&[1, 2, 3]), None);
	}

	#[test]
	fn test_bytes_add_bytes() {
		fn test_for(lhs: &[u8], rhs: &[u8], expected: &[u8]) {
			fn bytes_add_bytes_copy(lhs: &[u8], rhs: &[u8]) -> Vec<u8> {
				let mut lhs_vec = lhs.to_vec();
				bytes_add_bytes(&mut lhs_vec, rhs);
				lhs_vec
			}
			assert_eq!(
				bytes_add_bytes_copy(lhs, rhs).as_slice(),
				expected
			);
			// Changing lhs with rhs should not change the result.
			assert_eq!(
				bytes_add_bytes_copy(rhs, lhs).as_slice(),
				expected
			);
		}
		// 0 + 0 == 0
		test_for(
			&[0x00, 0x00, 0x00, 0x00],
			&[0x00, 0x00, 0x00, 0x00],
			&[0x00, 0x00, 0x00, 0x00],
		);
		// 0 + 0x42 == 0x42
		test_for(
			&[0x00, 0x00, 0x00, 0x00],
			&[0x00, 0x00, 0x00, 0x42],
			&[0x00, 0x00, 0x00, 0x42],
		);
		// u32::MAX + 1 == 0
		test_for(
			&[0xFF, 0xFF, 0xFF, 0xFF],
			&[0x00, 0x00, 0x00, 0x01],
			&[0x00, 0x00, 0x00, 0x00],
		);
		// 0 + u32::MAX == u32::MAX
		test_for(
			&[0x00, 0x00, 0x00, 0x00],
			&[0xFF, 0xFF, 0xFF, 0xFF],
			&[0xFF, 0xFF, 0xFF, 0xFF],
		);
		// 0x12345678 + 0x9ABCDEF0 = 0xACF13568
		test_for(
			&[0x12, 0x34, 0x56, 0x78],
			&[0x9A, 0xBC, 0xDE, 0xF0],
			&[0xAC, 0xF1, 0x35, 0x68],
		);
	}

	#[test]
	fn u32_and_bytes_conv() {
		fn test_for(val: u32, bytes: [u8; 4]) {
			assert_eq!(bytes4_to_u32(&u32_to_bytes4(val)), val);
			assert_eq!(u32_to_bytes4(bytes4_to_u32(&bytes)), bytes);
			assert_eq!(u32_to_bytes4(val), bytes);
		}
		test_for(
			0x00_00_00_00,
			[0x00, 0x00, 0x00, 0x00]
		);
		test_for(
			0xFF_FF_FF_FF,
			[0xFF, 0xFF, 0xFF, 0xFF]
		);
		test_for(
			0x00_00_00_01,
			[0x00, 0x00, 0x00, 0x01]
		);
		test_for(
			0x12_34_56_78,
			[0x12, 0x34, 0x56, 0x78]
		);
	}

	#[test]
	fn u64_and_bytes_conv() {
		fn test_for(val: u64, bytes: [u8; 8]) {
			assert_eq!(bytes8_to_u64(&u64_to_bytes8(val)), val);
			assert_eq!(u64_to_bytes8(bytes8_to_u64(&bytes)), bytes);
			assert_eq!(u64_to_bytes8(val), bytes);
		}
		// Test for 0
		test_for(
			0x00_00_00_00_00_00_00_00,
			[
				0x00, 0x00, 0x00, 0x00,
				0x00, 0x00, 0x00, 0x00,
			]
		);
		// Test for MAX
		test_for(
			0xFF_FF_FF_FF_FF_FF_FF_FF,
			[
				0xFF, 0xFF, 0xFF, 0xFF,
				0xFF, 0xFF, 0xFF, 0xFF,
			]
		);
		// Test for 1
		test_for(
			0x00_00_00_00_00_00_00_01,
			[
				0x00, 0x00, 0x00, 0x00,
				0x00, 0x00, 0x00, 0x01,
			]
		);
		// Test for unique bytes
		test_for(
			0x12_34_56_78_9A_BC_DE_F0,
			[
				0x12, 0x34, 0x56, 0x78,
				0x9A, 0xBC, 0xDE, 0xF0,
			]
		);
	}
}
