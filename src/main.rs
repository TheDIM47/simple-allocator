struct Alloc<'mem> {
    mem: &'mem mut [u8],
}

#[derive(Debug, PartialEq)]
pub struct OutOfMemory;

type AllocResult<T> = Result<T, OutOfMemory>;

impl<'mem> Alloc<'mem> {
    pub fn new(heap: &'mem mut [u8]) -> Self {
        Alloc { mem: heap }
    }

    pub fn alloc<'item, T>(&mut self, item: T) -> AllocResult<&'item mut T>
        where 'mem: 'item
    {
        self.waste_mem::<T>()?;

        unsafe { self.alloc_aligned(item) }
    }

    pub fn alloc_from_fn<'item, T>(&mut self, size: usize, f: impl Fn(usize) -> T) -> AllocResult<&'item mut [T]>
        where 'mem: 'item
    {
        self.waste_mem::<T>()?;

        let arr_ptr = self.mem as *mut [u8] as *mut [T];
        for i in 0..size {
            let _ = unsafe {
                self.alloc_aligned(f(i)).unwrap_unchecked()
            };
        };
        Ok(&mut unsafe { &mut *arr_ptr }[0..size])
    }

    // Note: self.mem must be aligned for T before call
    unsafe fn alloc_aligned<'item, T>(&mut self, item: T) -> AllocResult<&'item mut T>
        where 'mem: 'item
    {
        let required_size = core::mem::size_of::<T>();
        if self.mem.len() < required_size { return Err(OutOfMemory); }

        let item_ref = self.alloc_mem(required_size);

        let item_ptr = item_ref as *mut [u8] as *mut T;
        let item_ref = unsafe {
            core::ptr::write(item_ptr, item);
            &mut *item_ptr
        };

        Ok(item_ref)
    }

    fn alloc_mem(&mut self, size: usize) -> &mut [u8] {
        let mem = core::mem::take(&mut self.mem);
        let (item_ref, remaining_ref) = mem.split_at_mut(size);
        self.mem = remaining_ref;
        item_ref
    }

    fn calc_waste_bytes<T>(&mut self) -> usize {
        let alignment = core::mem::align_of::<T>();
        self.mem.as_ptr() as usize % alignment
    }

    fn waste_mem<T>(&mut self) -> AllocResult<usize> {
        let waste_bytes = self.calc_waste_bytes::<T>();
        if self.mem.len() < waste_bytes { return Err(OutOfMemory); }
        self.alloc_mem(waste_bytes);
        Ok(waste_bytes)
    }
}

fn main() {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alloc_bytes() {
        let mut heap: [u8; 4] = core::array::from_fn(|_| 0);
        let mut alloc = Alloc::new(&mut heap);
        let _ = alloc.alloc::<u8>(1);
        let _ = alloc.alloc::<u8>(2);
        let _ = alloc.alloc::<u8>(3);
        assert_eq!(heap, [1, 2, 3, 0])
    }

    #[test]
    fn alloc_i64s() {
        let mut heap: [u8; 32] = core::array::from_fn(|_| 0);
        let mut alloc = Alloc::new(&mut heap);
        let _ = alloc.alloc::<i64>(1);
        let _ = alloc.alloc::<i64>(2);
        let _ = alloc.alloc::<i64>(3);
        assert_eq!(heap, [
            1, 0, 0, 0, 0, 0, 0, 0,
            2, 0, 0, 0, 0, 0, 0, 0,
            3, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0,
        ])
    }

    #[test]
    fn alloc_aligned() {
        let mut heap: [u8; 16] = core::array::from_fn(|_| 0);
        let mut alloc = Alloc::new(&mut heap);

        let u8_ref = alloc.alloc::<u8>(1).unwrap();

        assert!(u8_ref as *mut u8 as usize % 2 == 0);

        let _ = alloc.alloc::<u16>(2);
        let _ = alloc.alloc::<u16>(3);

        assert_eq!(heap, [
            1, 0, 2, 0,
            3, 0, 0, 0,
            0, 0, 0, 0,
            0, 0, 0, 0,
        ])
    }

    #[test]
    fn alloc_out_of_mem() {
        let mut heap: [u8; 4] = core::array::from_fn(|_| 0);
        let mut alloc = Alloc::new(&mut heap);

        let u8_ref = alloc.alloc::<u8>(1).unwrap();

        assert!(u8_ref as *mut u8 as usize % 2 == 0);

        let result = alloc.alloc::<u32>(2);

        assert_eq!(result, Err(OutOfMemory))
    }

    #[test]
    fn alloc_fn() {
        let mut heap: [u8; 8] = core::array::from_fn(|_| 0);
        let mut alloc = Alloc::new(&mut heap);
        let _ = alloc.alloc_from_fn::<u8>(4, |i| (i + 1) as u8);
        assert_eq!(heap, [1, 2, 3, 4, 0, 0, 0, 0])
    }
}

