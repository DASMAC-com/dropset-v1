use core::mem::MaybeUninit;

use instruction_macros::{
    Pack,
    Unpack,
};
use price::OrderInfoArgs;
use solana_program_error::ProgramError;
use static_assertions::const_assert_eq;

use crate::{
    instructions::orders::private::UpToFive,
    state::user_order_sectors::{
        MAX_ORDERS,
        MAX_ORDERS_USIZE,
    },
};

#[repr(C)]
#[derive(Debug, Clone, Pack, Unpack, PartialEq, Eq)]
pub struct Orders {
    /// The number of elements representing real order arguments in [`Orders::order_args`].
    /// This value will always be less than or equal to the max number of orders [`MAX_ORDERS`].
    /// The remaining elements will be zero-initialized but cannot be accessed through the
    /// public [`Orders`] interface.
    num_orders: u8,
    /// Instruction data that isn't read is free, so it's simpler to always use [`MAX_ORDERS`]
    /// elements in the array and simply ignore elements with an index >= [`Orders::num_orders`]
    /// than to use a slice with a dynamic length.
    order_args: OrdersArray,
}

impl Orders {
    #[inline(always)]
    pub fn new<const N: usize>(orders: [OrderInfoArgs; N]) -> Self
    where
        [OrderInfoArgs; N]: UpToFive<N>,
    {
        let mut res: [MaybeUninit<OrderInfoArgs>; MAX_ORDERS_USIZE] =
            [const { MaybeUninit::uninit() }; MAX_ORDERS_USIZE];

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
                MAX_ORDERS_USIZE - N,
            );
        }

        Self {
            num_orders: N as u8,
            // Safety: The array has been fully initialized with `MAX_ORDERS` elements.
            order_args: OrdersArray(unsafe {
                core::mem::transmute::<
                    [MaybeUninit<OrderInfoArgs>; MAX_ORDERS_USIZE],
                    [OrderInfoArgs; MAX_ORDERS_USIZE],
                >(res)
            }),
        }
    }

    /// Exposes the order args elements as an owned iterator for indices 0..[`Self::num_orders`]`.
    #[inline(always)]
    pub fn into_order_args_iter(self) -> impl Iterator<Item = OrderInfoArgs> {
        let n = self.num_orders as usize;
        self.order_args.0.into_iter().take(n)
    }

    #[inline(always)]
    pub fn num_orders(&self) -> u8 {
        self.num_orders
    }
}

#[repr(transparent)]
#[derive(Debug, Clone, PartialEq, Eq)]
struct OrdersArray([OrderInfoArgs; MAX_ORDERS_USIZE]);

unsafe impl Pack for OrdersArray {
    type Packed = [u8; OrderInfoArgs::LEN * MAX_ORDERS_USIZE];

    /// # Safety
    ///
    /// Writes [`OrderInfoArgs::LEN`] bytes to `dst` [`MAX_ORDERS`] times, with each write
    /// destination offset increased by the amount written each time.
    #[inline(always)]
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

    #[inline(always)]
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
    #[inline(always)]
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

    #[inline(always)]
    fn unpack(data: &[u8]) -> Result<Self, solana_program_error::ProgramError> {
        if data.len() < Self::LEN {
            return Err(ProgramError::InvalidInstructionData);
        }

        // Safety: `data` has at least `Self::LEN` bytes.
        unsafe { Self::read_bytes(data.as_ptr()) }
    }
}

mod private {
    use super::*;

    // This sealed trait was written with the expectation that the max number of orders is 5.
    // If that changes, the trait needs to change to account for the different size.
    const_assert_eq!(MAX_ORDERS, 5);

    /// Marker trait: implemented only for arrays of length 0..=[`MAX_ORDERS`].
    pub trait UpToFive<const N: usize> {}

    impl UpToFive<0> for [OrderInfoArgs; 0] {}
    impl UpToFive<1> for [OrderInfoArgs; 1] {}
    impl UpToFive<2> for [OrderInfoArgs; 2] {}
    impl UpToFive<3> for [OrderInfoArgs; 3] {}
    impl UpToFive<4> for [OrderInfoArgs; 4] {}
    impl UpToFive<5> for [OrderInfoArgs; 5] {}
}
