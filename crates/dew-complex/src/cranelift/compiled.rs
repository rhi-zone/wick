//! Compiled function types for Cranelift JIT.

use cranelift_jit::JITModule;

/// A compiled complex function that returns a scalar.
pub struct CompiledComplexFn {
    pub(super) _module: JITModule,
    pub(super) func_ptr: *const u8,
    pub(super) param_count: usize,
}

unsafe impl Send for CompiledComplexFn {}
unsafe impl Sync for CompiledComplexFn {}

impl CompiledComplexFn {
    /// Calls the compiled function.
    pub fn call(&self, args: &[f32]) -> f32 {
        assert_eq!(args.len(), self.param_count, "wrong number of arguments");

        unsafe {
            match self.param_count {
                0 => jit_call!(self.func_ptr, args, f32, []),
                1 => jit_call!(self.func_ptr, args, f32, [0]),
                2 => jit_call!(self.func_ptr, args, f32, [0, 1]),
                3 => jit_call!(self.func_ptr, args, f32, [0, 1, 2]),
                4 => jit_call!(self.func_ptr, args, f32, [0, 1, 2, 3]),
                5 => jit_call!(self.func_ptr, args, f32, [0, 1, 2, 3, 4]),
                6 => jit_call!(self.func_ptr, args, f32, [0, 1, 2, 3, 4, 5]),
                7 => jit_call!(self.func_ptr, args, f32, [0, 1, 2, 3, 4, 5, 6]),
                8 => jit_call!(self.func_ptr, args, f32, [0, 1, 2, 3, 4, 5, 6, 7]),
                9 => jit_call!(self.func_ptr, args, f32, [0, 1, 2, 3, 4, 5, 6, 7, 8]),
                10 => jit_call!(self.func_ptr, args, f32, [0, 1, 2, 3, 4, 5, 6, 7, 8, 9]),
                11 => jit_call!(self.func_ptr, args, f32, [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10]),
                12 => jit_call!(
                    self.func_ptr,
                    args,
                    f32,
                    [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11]
                ),
                13 => jit_call!(
                    self.func_ptr,
                    args,
                    f32,
                    [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]
                ),
                14 => jit_call!(
                    self.func_ptr,
                    args,
                    f32,
                    [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13]
                ),
                15 => jit_call!(
                    self.func_ptr,
                    args,
                    f32,
                    [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14]
                ),
                16 => jit_call!(
                    self.func_ptr,
                    args,
                    f32,
                    [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15]
                ),
                _ => panic!("too many parameters (max 16)"),
            }
        }
    }
}

/// A compiled complex function that returns a complex value (two f32s).
/// Uses output pointer approach for reliable ABI handling.
pub struct CompiledComplexPairFn {
    pub(super) _module: JITModule,
    pub(super) func_ptr: *const u8,
    pub(super) param_count: usize,
}

unsafe impl Send for CompiledComplexPairFn {}
unsafe impl Sync for CompiledComplexPairFn {}

impl CompiledComplexPairFn {
    /// Calls the compiled function, returning a complex value as [re, im].
    pub fn call(&self, args: &[f32]) -> [f32; 2] {
        assert_eq!(args.len(), self.param_count, "wrong number of arguments");

        let mut output = [0.0f32; 2];
        let out_ptr = output.as_mut_ptr();

        unsafe {
            match self.param_count {
                0 => jit_call_outptr!(self.func_ptr, args, out_ptr, []),
                1 => jit_call_outptr!(self.func_ptr, args, out_ptr, [0]),
                2 => jit_call_outptr!(self.func_ptr, args, out_ptr, [0, 1]),
                3 => jit_call_outptr!(self.func_ptr, args, out_ptr, [0, 1, 2]),
                4 => jit_call_outptr!(self.func_ptr, args, out_ptr, [0, 1, 2, 3]),
                5 => jit_call_outptr!(self.func_ptr, args, out_ptr, [0, 1, 2, 3, 4]),
                6 => jit_call_outptr!(self.func_ptr, args, out_ptr, [0, 1, 2, 3, 4, 5]),
                7 => jit_call_outptr!(self.func_ptr, args, out_ptr, [0, 1, 2, 3, 4, 5, 6]),
                8 => jit_call_outptr!(self.func_ptr, args, out_ptr, [0, 1, 2, 3, 4, 5, 6, 7]),
                9 => jit_call_outptr!(self.func_ptr, args, out_ptr, [0, 1, 2, 3, 4, 5, 6, 7, 8]),
                10 => {
                    jit_call_outptr!(self.func_ptr, args, out_ptr, [0, 1, 2, 3, 4, 5, 6, 7, 8, 9])
                }
                11 => jit_call_outptr!(
                    self.func_ptr,
                    args,
                    out_ptr,
                    [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10]
                ),
                12 => jit_call_outptr!(
                    self.func_ptr,
                    args,
                    out_ptr,
                    [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11]
                ),
                13 => jit_call_outptr!(
                    self.func_ptr,
                    args,
                    out_ptr,
                    [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]
                ),
                14 => jit_call_outptr!(
                    self.func_ptr,
                    args,
                    out_ptr,
                    [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13]
                ),
                15 => jit_call_outptr!(
                    self.func_ptr,
                    args,
                    out_ptr,
                    [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14]
                ),
                16 => jit_call_outptr!(
                    self.func_ptr,
                    args,
                    out_ptr,
                    [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15]
                ),
                _ => panic!("too many parameters (max 16)"),
            };
        }
        output
    }
}
