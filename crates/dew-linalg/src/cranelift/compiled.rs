//! Compiled function structs for Cranelift JIT.

use cranelift_jit::JITModule;

/// A compiled linalg function.
pub struct CompiledLinalgFn {
    pub(super) _module: JITModule,
    pub(super) func_ptr: *const u8,
    pub(super) param_count: usize,
}

unsafe impl Send for CompiledLinalgFn {}
unsafe impl Sync for CompiledLinalgFn {}

impl CompiledLinalgFn {
    /// Calls the compiled function.
    /// All vector components are flattened into the args array.
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

/// A compiled linalg function that returns a Vec2 (two f32s).
/// Uses output pointer approach for reliable ABI handling.
pub struct CompiledVec2Fn {
    pub(super) _module: JITModule,
    pub(super) func_ptr: *const u8,
    pub(super) param_count: usize,
}

unsafe impl Send for CompiledVec2Fn {}
unsafe impl Sync for CompiledVec2Fn {}

impl CompiledVec2Fn {
    /// Calls the compiled function, returning a Vec2 as [x, y].
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

/// A compiled linalg function that returns a Vec3 (three f32s).
/// Uses output pointer approach for reliable ABI handling.
pub struct CompiledVec3Fn {
    pub(super) _module: JITModule,
    pub(super) func_ptr: *const u8,
    pub(super) param_count: usize,
}

unsafe impl Send for CompiledVec3Fn {}
unsafe impl Sync for CompiledVec3Fn {}

impl CompiledVec3Fn {
    /// Calls the compiled function, returning a Vec3 as [x, y, z].
    pub fn call(&self, args: &[f32]) -> [f32; 3] {
        assert_eq!(args.len(), self.param_count, "wrong number of arguments");

        let mut output = [0.0f32; 3];
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

/// A compiled linalg function that returns a Vec4 (four f32s).
/// Uses output pointer approach for reliable ABI handling.
#[cfg(feature = "4d")]
pub struct CompiledVec4Fn {
    pub(super) _module: JITModule,
    pub(super) func_ptr: *const u8,
    pub(super) param_count: usize,
}

#[cfg(feature = "4d")]
unsafe impl Send for CompiledVec4Fn {}
#[cfg(feature = "4d")]
unsafe impl Sync for CompiledVec4Fn {}

#[cfg(feature = "4d")]
impl CompiledVec4Fn {
    /// Calls the compiled function, returning a Vec4 as [x, y, z, w].
    pub fn call(&self, args: &[f32]) -> [f32; 4] {
        assert_eq!(args.len(), self.param_count, "wrong number of arguments");

        let mut output = [0.0f32; 4];
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
                17 => jit_call_outptr!(
                    self.func_ptr,
                    args,
                    out_ptr,
                    [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]
                ),
                18 => jit_call_outptr!(
                    self.func_ptr,
                    args,
                    out_ptr,
                    [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17]
                ),
                19 => jit_call_outptr!(
                    self.func_ptr,
                    args,
                    out_ptr,
                    [
                        0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18
                    ]
                ),
                20 => jit_call_outptr!(
                    self.func_ptr,
                    args,
                    out_ptr,
                    [
                        0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19
                    ]
                ),
                _ => panic!("too many parameters (max 20)"),
            };
        }
        output
    }
}

/// A compiled linalg function that returns a Mat2 (four f32s).
/// Uses output pointer approach for reliable ABI handling.
pub struct CompiledMat2Fn {
    pub(super) _module: JITModule,
    pub(super) func_ptr: *const u8,
    pub(super) param_count: usize,
}

unsafe impl Send for CompiledMat2Fn {}
unsafe impl Sync for CompiledMat2Fn {}

impl CompiledMat2Fn {
    /// Calls the compiled function, returning a Mat2 as [c0r0, c0r1, c1r0, c1r1].
    pub fn call(&self, args: &[f32]) -> [f32; 4] {
        assert_eq!(args.len(), self.param_count, "wrong number of arguments");

        let mut output = [0.0f32; 4];
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

/// A compiled linalg function that returns a Mat3 (nine f32s).
/// Uses output pointer approach for reliable ABI handling.
#[cfg(feature = "3d")]
pub struct CompiledMat3Fn {
    pub(super) _module: JITModule,
    pub(super) func_ptr: *const u8,
    pub(super) param_count: usize,
}

#[cfg(feature = "3d")]
unsafe impl Send for CompiledMat3Fn {}
#[cfg(feature = "3d")]
unsafe impl Sync for CompiledMat3Fn {}

#[cfg(feature = "3d")]
impl CompiledMat3Fn {
    /// Calls the compiled function, returning a Mat3 as 9 f32s (column-major).
    pub fn call(&self, args: &[f32]) -> [f32; 9] {
        assert_eq!(args.len(), self.param_count, "wrong number of arguments");

        let mut output = [0.0f32; 9];
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
                17 => jit_call_outptr!(
                    self.func_ptr,
                    args,
                    out_ptr,
                    [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]
                ),
                18 => jit_call_outptr!(
                    self.func_ptr,
                    args,
                    out_ptr,
                    [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17]
                ),
                _ => panic!("too many parameters (max 18)"),
            };
        }
        output
    }
}

/// A compiled linalg function that returns a Mat4 (sixteen f32s).
/// Uses output pointer approach for reliable ABI handling.
#[cfg(feature = "4d")]
pub struct CompiledMat4Fn {
    pub(super) _module: JITModule,
    pub(super) func_ptr: *const u8,
    pub(super) param_count: usize,
}

#[cfg(feature = "4d")]
unsafe impl Send for CompiledMat4Fn {}
#[cfg(feature = "4d")]
unsafe impl Sync for CompiledMat4Fn {}

#[cfg(feature = "4d")]
impl CompiledMat4Fn {
    /// Calls the compiled function, returning a Mat4 as 16 f32s (column-major).
    pub fn call(&self, args: &[f32]) -> [f32; 16] {
        assert_eq!(args.len(), self.param_count, "wrong number of arguments");

        let mut output = [0.0f32; 16];
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
                17 => jit_call_outptr!(
                    self.func_ptr,
                    args,
                    out_ptr,
                    [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]
                ),
                18 => jit_call_outptr!(
                    self.func_ptr,
                    args,
                    out_ptr,
                    [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17]
                ),
                19 => jit_call_outptr!(
                    self.func_ptr,
                    args,
                    out_ptr,
                    [
                        0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18
                    ]
                ),
                20 => jit_call_outptr!(
                    self.func_ptr,
                    args,
                    out_ptr,
                    [
                        0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19
                    ]
                ),
                21 => jit_call_outptr!(
                    self.func_ptr,
                    args,
                    out_ptr,
                    [
                        0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20
                    ]
                ),
                22 => jit_call_outptr!(
                    self.func_ptr,
                    args,
                    out_ptr,
                    [
                        0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
                        21
                    ]
                ),
                23 => jit_call_outptr!(
                    self.func_ptr,
                    args,
                    out_ptr,
                    [
                        0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
                        21, 22
                    ]
                ),
                24 => jit_call_outptr!(
                    self.func_ptr,
                    args,
                    out_ptr,
                    [
                        0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
                        21, 22, 23
                    ]
                ),
                25 => jit_call_outptr!(
                    self.func_ptr,
                    args,
                    out_ptr,
                    [
                        0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
                        21, 22, 23, 24
                    ]
                ),
                26 => jit_call_outptr!(
                    self.func_ptr,
                    args,
                    out_ptr,
                    [
                        0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
                        21, 22, 23, 24, 25
                    ]
                ),
                27 => jit_call_outptr!(
                    self.func_ptr,
                    args,
                    out_ptr,
                    [
                        0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
                        21, 22, 23, 24, 25, 26
                    ]
                ),
                28 => jit_call_outptr!(
                    self.func_ptr,
                    args,
                    out_ptr,
                    [
                        0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
                        21, 22, 23, 24, 25, 26, 27
                    ]
                ),
                29 => jit_call_outptr!(
                    self.func_ptr,
                    args,
                    out_ptr,
                    [
                        0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
                        21, 22, 23, 24, 25, 26, 27, 28
                    ]
                ),
                30 => jit_call_outptr!(
                    self.func_ptr,
                    args,
                    out_ptr,
                    [
                        0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
                        21, 22, 23, 24, 25, 26, 27, 28, 29
                    ]
                ),
                31 => jit_call_outptr!(
                    self.func_ptr,
                    args,
                    out_ptr,
                    [
                        0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
                        21, 22, 23, 24, 25, 26, 27, 28, 29, 30
                    ]
                ),
                32 => jit_call_outptr!(
                    self.func_ptr,
                    args,
                    out_ptr,
                    [
                        0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
                        21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31
                    ]
                ),
                _ => panic!("too many parameters (max 32)"),
            };
        }
        output
    }
}
