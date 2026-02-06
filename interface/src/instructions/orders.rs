use core::mem::MaybeUninit;

use instruction_macros::{
    Pack,
    Unpack,
};
use price::OrderInfoArgs;
use solana_program_error::ProgramError;
use static_assertions::const_assert_eq;

use crate::state::user_order_sectors::MAX_ORDERS;

#[repr(C)]
#[derive(Debug, Clone, Pack, Unpack, PartialEq, Eq)]
pub struct Orders {
    /// The number of elements representing real order arguments in [`NewOrders::order_args`].
    /// This value will always be less than or equal to the max number of orders [`MAX_ORDERS`].
    /// The remaining elements will be zero-initialized but cannot be accessed through the
    /// public [`Orders`] interface.
    pub num_orders: u8,
    /// Instruction data that isn't read is free, so it's simpler to always use [`MAX_ORDERS`]
    /// elements in the array and simply ignore elements with an index >= [`NewOrders::num_orders`]
    /// than to use a slice with a dynamic length.
    order_args: OrdersArray,
}

impl Orders {
    pub fn new<const N: usize>(orders: [OrderInfoArgs; N]) -> Self {
        const {
            // Number of orders must be <= MAX_ORDERS.
            assert!(N <= MAX_ORDERS as usize);
        }

        let mut res: [MaybeUninit<OrderInfoArgs>; MAX_ORDERS as usize] =
            [const { MaybeUninit::uninit() }; MAX_ORDERS as usize];

        unsafe {
            // Copy the orders passed in. This initializes `res[0..N]`.
            //
            // Safety:
            // - `orders` is valid for `N` reads.
            // - `res` is valid for `MAX_ORDERS` writes, and `MAX_ORDERS` >= `N`.
            // - Both pointers are aligned and do not overlap.
            core::ptr::copy_nonoverlapping(
                orders.as_ptr(),
                res.as_mut_ptr() as *mut OrderInfoArgs,
                N,
            );

            // Write zeros to the remaining elements. This initializes `res[N..MAX_ORDERS]`.
            //
            // Safety:
            // - `res.as_mut_ptr().add(N)` is valid for up to `MAX_ORDERS - N` writes and `N` is
            //   guaranteed to be <= `MAX_ORDERS`.
            // - Zero is a valid value for all field types.
            core::ptr::write_bytes(
                (res.as_mut_ptr() as *mut OrderInfoArgs).add(N),
                0u8,
                MAX_ORDERS as usize - N,
            );
        }

        Self {
            num_orders: N as u8,
            // Safety: The array has been fully initialized with `MAX_ORDERS` elements.
            order_args: OrdersArray(unsafe {
                core::mem::transmute::<
                    [MaybeUninit<OrderInfoArgs>; MAX_ORDERS as usize],
                    [OrderInfoArgs; MAX_ORDERS as usize],
                >(res)
            }),
        }
    }

    /// Exposes the valid order args elements as a slice view from 0..[`Self::num_orders`]`.
    pub fn order_args(&self) -> &[OrderInfoArgs] {
        &self.order_args.0[..self.num_orders as usize]
    }
}

#[repr(transparent)]
#[derive(Debug, Clone, PartialEq, Eq)]
struct OrdersArray([OrderInfoArgs; MAX_ORDERS as usize]);

unsafe impl Pack for OrdersArray {
    type Packed = [u8; OrderInfoArgs::LEN * MAX_ORDERS as usize];

    /// # Safety
    ///
    /// Writes [`OrderInfoArgs::LEN`] bytes to `dst` [`MAX_ORDERS`] times, with each write
    /// destination offset increased by the amount written each time.
    unsafe fn write_bytes(&self, dst: *mut u8) {
        // This implementation was written with the expectation that the max number of orders is 5.
        // If that changes, the implementation needs to change to account for the different size.
        const_assert_eq!(MAX_ORDERS, 5);

        let array = &self.0;
        array[0].write_bytes(dst);
        array[1].write_bytes(dst.add(OrderInfoArgs::LEN));
        array[2].write_bytes(dst.add(OrderInfoArgs::LEN * 2));
        array[3].write_bytes(dst.add(OrderInfoArgs::LEN * 3));
        array[4].write_bytes(dst.add(OrderInfoArgs::LEN * 4));
    }

    fn pack(&self) -> Self::Packed {
        let mut data: [MaybeUninit<u8>; Self::LEN] = [MaybeUninit::uninit(); Self::LEN];
        let dst = data.as_mut_ptr() as *mut u8;
        // Safety: `dst` points to `Self::LEN` contiguous, writable bytes.
        unsafe { self.write_bytes(dst) };

        // Safety: All bytes are initialized during the construction above.
        unsafe { *(data.as_ptr() as *const [u8; Self::LEN]) }
    }
}

unsafe impl Unpack for OrdersArray {
    /// # Safety (implementor)
    ///
    /// - Exactly [`Orders::LEN`] bytes are read from `src`.
    /// - `src` is read from as an unaligned pointer.
    ///
    /// # Safety (caller)
    ///
    /// Caller must guarantee `src` points to at least [`Self::LEN`] bytes of readable memory.
    unsafe fn read_bytes(src: *const u8) -> Result<Self, solana_program_error::ProgramError> {
        // This implementation was written with the expectation that the max number of orders is 5.
        // If that changes, the implementation needs to change to account for the different size.
        const_assert_eq!(MAX_ORDERS, 5);

        Ok(Self([
            OrderInfoArgs::read_bytes(src)?,
            OrderInfoArgs::read_bytes(src.add(OrderInfoArgs::LEN))?,
            OrderInfoArgs::read_bytes(src.add(OrderInfoArgs::LEN * 2))?,
            OrderInfoArgs::read_bytes(src.add(OrderInfoArgs::LEN * 3))?,
            OrderInfoArgs::read_bytes(src.add(OrderInfoArgs::LEN * 4))?,
        ]))
    }

    fn unpack(data: &[u8]) -> Result<Self, solana_program_error::ProgramError> {
        if data.len() < Self::LEN {
            return Err(ProgramError::InvalidInstructionData);
        }

        // Safety: `data` has at least `Self::LEN` bytes.
        unsafe { Self::read_bytes(data.as_ptr()) }
    }
}
