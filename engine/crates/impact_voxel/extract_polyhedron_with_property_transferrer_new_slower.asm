.section .text.<impact_voxel::object::VoxelObject>::extract_polyhedron_with_property_transferrer_c,"ax",@progbits
	.globl	<impact_voxel::object::VoxelObject>::extract_polyhedron_with_property_transferrer_c
	.p2align	4
.type	<impact_voxel::object::VoxelObject>::extract_polyhedron_with_property_transferrer_c,@function
<impact_voxel::object::VoxelObject>::extract_polyhedron_with_property_transferrer_c:
		// crates/impact_voxel/src/object/extraction.rs:618
		pub fn extract_polyhedron_with_property_transferrer_c(
	.cfi_startproc
	.cfi_personality 155, DW.ref.rust_eh_personality
	.cfi_lsda 27, .Lexception95
	pushq %rbp
	.cfi_def_cfa_offset 16
	pushq %r15
	.cfi_def_cfa_offset 24
	pushq %r14
	.cfi_def_cfa_offset 32
	pushq %r13
	.cfi_def_cfa_offset 40
	pushq %r12
	.cfi_def_cfa_offset 48
	pushq %rbx
	.cfi_def_cfa_offset 56
	subq $2472, %rsp
	.cfi_def_cfa_offset 2528
	.cfi_offset %rbx, -56
	.cfi_offset %r12, -48
	.cfi_offset %r13, -40
	.cfi_offset %r14, -32
	.cfi_offset %r15, -24
	.cfi_offset %rbp, -16
		// crates/impact_voxel/src/object/extraction.rs:625
		self.extract_polyhedron_with_property_transferrer(
	vmovaps (%rcx), %xmm0
	vmovaps 16(%rcx), %xmm1
	vmovss .LCPI226_2(%rip), %xmm8
	xorl %eax, %eax
	movq %rdx, 384(%rsp)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/array/mod.rs:477
		(&*old).clone()
	movq 312(%rsi), %rdx
	movq $-1, %rcx
	movq %r9, 32(%rsp)
	vxorps %xmm7, %xmm7, %xmm7
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:1038
		if other < self { self } else { other }
	vmovdqu 296(%rsi), %xmm6
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/../../stdarch/crates/core_arch/src/x86/sse.rs:62
		unsafe { simd_sub(a, b) }
	vaddps .LCPI226_0(%rip){1to4}, %xmm0, %xmm3
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/../../stdarch/crates/core_arch/src/x86/sse.rs:36
		unsafe { simd_add(a, b) }
	vaddps .LCPI226_1(%rip){1to4}, %xmm1, %xmm0
	vshufpd $1, %xmm3, %xmm3, %xmm1
	vroundps $9, %xmm3, %xmm3
	vroundss $9, %xmm1, %xmm1, %xmm2
	vxorps %xmm1, %xmm1, %xmm1
	vmaxps %xmm7, %xmm3, %xmm7
	vmaxss %xmm1, %xmm2, %xmm4
	vcmpunordps %xmm7, %xmm7, %k1
	vcvttss2usi %xmm4, %r10
	vucomiss %xmm1, %xmm4
	vmovaps %xmm3, %xmm7 {%k1}
	cmovbq %rax, %r10
	vucomiss %xmm8, %xmm4
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/num/f32.rs:1811
		intrinsics::ceilf32(x)
	vroundps $10, %xmm0, %xmm4
		// crates/impact_voxel/src/object/intersection.rs:778
		range.end = range.end.min(upper_corner[dim].ceil() as usize);
	vshufpd $1, %xmm0, %xmm0, %xmm0
	vmovshdup %xmm4, %xmm5
	vcvttss2usi %xmm4, %r11
	cmovaq %rcx, %r10
	vcvttss2usi %xmm5, %r9
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:1038
		if other < self { self } else { other }
	cmpq %rdx, %r10
	cmovbeq %rdx, %r10
		// crates/impact_voxel/src/object/intersection.rs:778
		range.end = range.end.min(upper_corner[dim].ceil() as usize);
	vucomiss %xmm1, %xmm5
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/array/mod.rs:477
		(&*old).clone()
	movq 320(%rsi), %rdx
		// crates/impact_voxel/src/object/intersection.rs:778
		range.end = range.end.min(upper_corner[dim].ceil() as usize);
	cmovbq %rax, %r9
	vucomiss %xmm8, %xmm5
	cmovaq %rcx, %r9
	vucomiss %xmm1, %xmm4
	vmovq %r9, %xmm5
	vcvttss2usi %xmm7, %r9
	cmovbq %rax, %r11
	vucomiss %xmm8, %xmm4
	cmovaq %rcx, %r11
	vucomiss %xmm1, %xmm7
	vmovq %r11, %xmm4
	vpunpcklqdq %xmm5, %xmm4, %xmm4
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:1077
		if other < self { other } else { self }
	vmovdqu 280(%rsi), %xmm5
	cmovbq %rax, %r9
	vucomiss %xmm8, %xmm7
	cmovaq %rcx, %r9
	vpunpckhqdq %xmm6, %xmm5, %xmm3
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:1038
		if other < self { self } else { other }
	vpunpcklqdq %xmm6, %xmm5, %xmm5
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/num/f32.rs:1811
		intrinsics::ceilf32(x)
	vroundss $10, %xmm0, %xmm0, %xmm6
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:1077
		if other < self { other } else { self }
	vpminuq %xmm3, %xmm4, %xmm3
	vmovshdup %xmm7, %xmm4
	vmovq %r9, %xmm7
	vcvttss2usi %xmm4, %r11
	vucomiss %xmm1, %xmm4
		// crates/impact_voxel/src/object/intersection.rs:778
		range.end = range.end.min(upper_corner[dim].ceil() as usize);
	vcvttss2usi %xmm6, %r9
	cmovbq %rax, %r11
	vucomiss %xmm8, %xmm4
	cmovaq %rcx, %r11
	vucomiss %xmm1, %xmm6
	vmovq %r11, %xmm4
	vpunpcklqdq %xmm4, %xmm7, %xmm4
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:1038
		if other < self { self } else { other }
	vpmaxuq %xmm5, %xmm4, %xmm0
		// crates/impact_voxel/src/object/intersection.rs:778
		range.end = range.end.min(upper_corner[dim].ceil() as usize);
	cmovbq %rax, %r9
	vucomiss %xmm8, %xmm6
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:1916
		fn lt(&self, other: &Self) -> bool { *self <  *other }
	vpcmpltuq %xmm3, %xmm0, %k0
	kmovd %k0, %eax
		// crates/impact_voxel/src/object/intersection.rs:778
		range.end = range.end.min(upper_corner[dim].ceil() as usize);
	cmovaq %rcx, %r9
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:1077
		if other < self { other } else { self }
	cmpq %rdx, %r9
	cmovaeq %rdx, %r9
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/iter/macros.rs:332
		if f(x) {
	testb $1, %al
	je .LBB226_25
	kshiftrb $1, %k0, %k0
	kmovd %k0, %eax
	testb $1, %al
	je .LBB226_25
	cmpq %r9, %r10
	jae .LBB226_25
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/num/uint_macros.rs:3757
		let r = self % rhs;
	vpandq .LCPI226_3(%rip){1to2}, %xmm3, %xmm2
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/num/uint_macros.rs:3756
		let d = self / rhs;
	movq %r9, %rax
	shrq $4, %rax
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/num/uint_macros.rs:3758
		if r > 0 {
	xorl %ecx, %ecx
	testb $15, %r9b
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/num/uint_macros.rs:3756
		let d = self / rhs;
	vpsrlq $4, %xmm3, %xmm1
		// crates/impact_voxel/src/object.rs:3237
		let start = voxel_range.start / CHUNK_SIZE;
	vpsrlq $4, %xmm0, %xmm3
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/num/uint_macros.rs:3758
		if r > 0 {
	vpxor %xmm0, %xmm0, %xmm0
	movq %r8, 632(%rsp)
	movq %rsi, 88(%rsp)
	movq %rdi, 576(%rsp)
	setne %cl
		// crates/impact_voxel/src/object.rs:3237
		let start = voxel_range.start / CHUNK_SIZE;
	shrq $4, %r10
	xorl %r13d, %r13d
	vmovdqu %ymm3, 1344(%rsp)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/num/uint_macros.rs:3758
		if r > 0 {
	addq %rax, %rcx
	movq %r10, 296(%rsp)
	movq %rcx, 304(%rsp)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/iter/range.rs:990
		if self.start < self.end {
	subq %r10, %rcx
	cmovbq %r13, %rcx
	movq %rcx, 872(%rsp)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:1916
		fn lt(&self, other: &Self) -> bool { *self <  *other }
	vmovq %xmm3, %rcx
	movq %rcx, 408(%rsp)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/num/uint_macros.rs:3758
		if r > 0 {
	vpcmpgtq %xmm0, %xmm2, %xmm0
	vpsubq %xmm0, %xmm1, %xmm0
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:1916
		fn lt(&self, other: &Self) -> bool { *self <  *other }
	vmovq %xmm0, %rax
	vmovdqu %ymm0, 1312(%rsp)
	movq %rax, 592(%rsp)
	cmpq %rax, %rcx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/iter/range.rs:900
		if self.start < self.end {
	jae .LBB226_18
	vmovdqu 1312(%rsp), %ymm0
	xorl %r13d, %r13d
	vpextrq $1, %xmm0, %rax
	vmovdqu 1344(%rsp), %ymm0
	movq %rax, (%rsp)
	vpextrq $1, %xmm0, %rcx
	movq %rcx, 152(%rsp)
	cmpq %rax, %rcx
	jae .LBB226_18
	movq 304(%rsp), %rax
	cmpq %rax, 296(%rsp)
	jae .LBB226_18
	movq 88(%rsp), %rax
	movq 408(%rsp), %rdx
	vpbroadcastq .LCPI226_5(%rip), %zmm1
	vpbroadcastq .LCPI226_6(%rip), %zmm2
	vpbroadcastq .LCPI226_7(%rip), %zmm3
	vpbroadcastb .LCPI226_21(%rip), %xmm4
	vpbroadcastq .LCPI226_9(%rip), %zmm5
	xorl %r13d, %r13d
	movq 216(%rax), %r8
	movq 208(%rax), %rdi
	movq 8(%rax), %r9
	movq 16(%rax), %rsi
	movq 296(%rsp), %rax
	movq %r8, %r11
	imulq 152(%rsp), %r11
	vpbroadcastq %rax, %zmm0
	vpaddq .LCPI226_4(%rip), %zmm0, %zmm0
	movq %rdi, %rcx
	imulq %rdx, %rcx
	movq %rax, %r10
	notq %r10
	addq 304(%rsp), %r10
	movq %rdi, 312(%rsp)
	addq %rcx, %r11
	xorl %ecx, %ecx
	leaq (%r11,%rax), %r15
	leaq 9(%r9), %rax
	movq %r15, %r14
	negq %r14
	movq %r15, 200(%rsp)
	.p2align	4
.LBB226_7:
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/num/uint_macros.rs:971
		intrinsics::unchecked_add(self, rhs)
	movq %rdi, %rbx
	imulq %rcx, %rbx
	addq 200(%rsp), %rbx
	movq 152(%rsp), %r12
	movq %rcx, 264(%rsp)
	leaq 1(%rdx), %rcx
	imulq %rdi, %rdx
	movq %r11, 184(%rsp)
	movq %r14, 560(%rsp)
	movq %r15, 248(%rsp)
	movq %rcx, 320(%rsp)
	movq %rdx, 80(%rsp)
	movq %rbx, 48(%rsp)
	xorl %ebx, %ebx
	.p2align	4
.LBB226_8:
	cmpq %r15, %rsi
	movq %r15, %rbp
	movq %r8, %rcx
	cmovaq %rsi, %rbp
	addq %r14, %rbp
	cmpq %rbp, %r10
	cmovbq %r10, %rbp
	imulq %rbx, %rcx
	addq 48(%rsp), %rcx
	cmpq %rcx, %rsi
	movq %rcx, %rdi
	cmovaq %rsi, %rdi
	subq %rcx, %rdi
	cmpq %rdi, %r10
	cmovbq %r10, %rdi
	incq %rdi
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/iter/range.rs:900
		if self.start < self.end {
	cmpq $33, %rdi
	jae .LBB226_10
	movq 296(%rsp), %rdi
	jmp .LBB226_13
	.p2align	4
.LBB226_10:
	movq %r12, %rcx
	imulq %r8, %rcx
	addq 80(%rsp), %rcx
	movl %edi, %eax
	andl $31, %eax
	movl $32, %edx
	notq %rbp
	vmovq %r13, %xmm6
	vpxor %xmm11, %xmm11, %xmm11
	vpxor %xmm12, %xmm12, %xmm12
	vpxor %xmm13, %xmm13, %xmm13
	vmovdqa64 %zmm0, %zmm14
	cmoveq %rdx, %rax
	subq %rax, %rdi
	addq 296(%rsp), %rdi
	addq %rax, %rbp
	vpbroadcastq %rcx, %zmm7
	vpaddq %zmm1, %zmm7, %zmm8
	vpaddq %zmm2, %zmm7, %zmm9
	vpaddq %zmm3, %zmm7, %zmm10
	.p2align	4
.LBB226_11:
	vpaddq %zmm14, %zmm7, %zmm15
	vmovq %xmm15, %rcx
	vpextrq $1, %xmm15, %r13
	vextracti32x4 $1, %ymm15, %xmm16
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/index.rs:272
		&(*slice)[self]
	shlq $4, %rcx
	shlq $4, %r13
	movzbl 9(%r9,%rcx), %eax
	vpextrq $1, %xmm16, %rcx
	shlq $4, %rcx
	vmovd %eax, %xmm17
	vpinsrb $1, 9(%r9,%r13), %xmm17, %xmm17
	vmovq %xmm16, %rax
	vextracti32x4 $2, %zmm15, %xmm16
	vextracti32x4 $3, %zmm15, %xmm15
	shlq $4, %rax
	vpinsrb $2, 9(%r9,%rax), %xmm17, %xmm17
	vpextrq $1, %xmm16, %rax
	shlq $4, %rax
	vpinsrb $3, 9(%r9,%rcx), %xmm17, %xmm17
	vmovq %xmm16, %rcx
	shlq $4, %rcx
	vpinsrb $4, 9(%r9,%rcx), %xmm17, %xmm16
	vmovq %xmm15, %rcx
	vpaddq %zmm8, %zmm14, %zmm17
	shlq $4, %rcx
	vpinsrb $5, 9(%r9,%rax), %xmm16, %xmm16
	vpextrq $1, %xmm15, %rax
	shlq $4, %rax
	vpinsrb $6, 9(%r9,%rcx), %xmm16, %xmm15
	vmovq %xmm17, %rcx
	vextracti32x4 $1, %ymm17, %xmm16
	shlq $4, %rcx
	movzbl 9(%r9,%rcx), %ecx
	vpinsrb $7, 9(%r9,%rax), %xmm15, %xmm15
	vpextrq $1, %xmm17, %rax
	shlq $4, %rax
	vmovd %ecx, %xmm18
	vpinsrb $1, 9(%r9,%rax), %xmm18, %xmm18
	vmovq %xmm16, %rcx
	vpextrq $1, %xmm16, %rax
	vextracti32x4 $2, %zmm17, %xmm16
	shlq $4, %rcx
	shlq $4, %rax
	vpinsrb $2, 9(%r9,%rcx), %xmm18, %xmm18
	vmovq %xmm16, %rcx
	shlq $4, %rcx
		// crates/impact_voxel/src/object/extraction.rs:684
		if let VoxelChunk::NonUniform(_) = chunk {
	vpcmpgtb %xmm15, %xmm4, %k0
	vpmovm2q %k0, %zmm15
	vpsubq %zmm15, %zmm6, %zmm6
	vpinsrb $3, 9(%r9,%rax), %xmm18, %xmm18
	vpextrq $1, %xmm16, %rax
	vextracti32x4 $3, %zmm17, %xmm16
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/index.rs:272
		&(*slice)[self]
	shlq $4, %rax
	vpinsrb $4, 9(%r9,%rcx), %xmm18, %xmm17
	vmovq %xmm16, %rcx
	shlq $4, %rcx
	vpinsrb $5, 9(%r9,%rax), %xmm17, %xmm17
	vpextrq $1, %xmm16, %rax
	vpaddq %zmm9, %zmm14, %zmm16
	shlq $4, %rax
	vextracti32x4 $1, %ymm16, %xmm18
	vpinsrb $6, 9(%r9,%rcx), %xmm17, %xmm17
	vmovq %xmm16, %rcx
	shlq $4, %rcx
	movzbl 9(%r9,%rcx), %ecx
	vpinsrb $7, 9(%r9,%rax), %xmm17, %xmm17
	vpextrq $1, %xmm16, %rax
	shlq $4, %rax
	vmovd %ecx, %xmm19
	vpinsrb $1, 9(%r9,%rax), %xmm19, %xmm19
	vmovq %xmm18, %rcx
	vpextrq $1, %xmm18, %rax
	shlq $4, %rcx
	shlq $4, %rax
	vpinsrb $2, 9(%r9,%rcx), %xmm19, %xmm18
	vextracti32x4 $2, %zmm16, %xmm19
	vextracti32x4 $3, %zmm16, %xmm16
		// crates/impact_voxel/src/object/extraction.rs:684
		if let VoxelChunk::NonUniform(_) = chunk {
	vpcmpgtb %xmm17, %xmm4, %k1
	vpmovm2q %k1, %zmm15
	vpsubq %zmm15, %zmm11, %zmm11
	vpinsrb $3, 9(%r9,%rax), %xmm18, %xmm18
	vmovq %xmm19, %rax
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/index.rs:272
		&(*slice)[self]
	shlq $4, %rax
	vpinsrb $4, 9(%r9,%rax), %xmm18, %xmm18
	vpextrq $1, %xmm19, %rax
	shlq $4, %rax
	vpinsrb $5, 9(%r9,%rax), %xmm18, %xmm18
	vmovq %xmm16, %rax
	shlq $4, %rax
	vpinsrb $6, 9(%r9,%rax), %xmm18, %xmm18
	vpextrq $1, %xmm16, %rax
	vpaddq %zmm10, %zmm14, %zmm16
	vpaddq %zmm5, %zmm14, %zmm14
	shlq $4, %rax
	vextracti32x4 $1, %ymm16, %xmm20
	vpinsrb $7, 9(%r9,%rax), %xmm18, %xmm18
	vmovq %xmm16, %rax
	shlq $4, %rax
	movzbl 9(%r9,%rax), %eax
	vmovd %eax, %xmm19
	vpextrq $1, %xmm16, %rax
	shlq $4, %rax
		// crates/impact_voxel/src/object/extraction.rs:684
		if let VoxelChunk::NonUniform(_) = chunk {
	vpcmpgtb %xmm18, %xmm4, %k2
	vpinsrb $1, 9(%r9,%rax), %xmm19, %xmm19
	vmovq %xmm20, %rax
	vpmovm2q %k2, %zmm15
	vpsubq %zmm15, %zmm12, %zmm12
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/index.rs:272
		&(*slice)[self]
	shlq $4, %rax
	vpinsrb $2, 9(%r9,%rax), %xmm19, %xmm19
	vpextrq $1, %xmm20, %rax
	vextracti32x4 $2, %zmm16, %xmm20
	vextracti32x4 $3, %zmm16, %xmm16
	shlq $4, %rax
	vpinsrb $3, 9(%r9,%rax), %xmm19, %xmm19
	vmovq %xmm20, %rax
	shlq $4, %rax
	vpinsrb $4, 9(%r9,%rax), %xmm19, %xmm19
	vpextrq $1, %xmm20, %rax
	shlq $4, %rax
	vpinsrb $5, 9(%r9,%rax), %xmm19, %xmm19
	vmovq %xmm16, %rax
	shlq $4, %rax
	vpinsrb $6, 9(%r9,%rax), %xmm19, %xmm19
	vpextrq $1, %xmm16, %rax
	shlq $4, %rax
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/iter/range.rs:900
		if self.start < self.end {
	addq $32, %rbp
	vpinsrb $7, 9(%r9,%rax), %xmm19, %xmm16
		// crates/impact_voxel/src/object/extraction.rs:684
		if let VoxelChunk::NonUniform(_) = chunk {
	vpcmpgtb %xmm16, %xmm4, %k3
	vpmovm2q %k3, %zmm15
	vpsubq %zmm15, %zmm13, %zmm13
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/iter/range.rs:900
		if self.start < self.end {
	jne .LBB226_11
	vpaddq %zmm6, %zmm11, %zmm6
	vpaddq %zmm12, %zmm13, %zmm7
	leaq 9(%r9), %rax
	vpaddq %zmm6, %zmm7, %zmm6
	vextracti64x4 $1, %zmm6, %ymm7
	vpaddq %zmm7, %zmm6, %zmm6
	vextracti128 $1, %ymm6, %xmm7
	vpaddq %xmm7, %xmm6, %xmm6
	vpshufd $238, %xmm6, %xmm7
	vpaddq %xmm7, %xmm6, %xmm6
	vmovq %xmm6, %r13
.LBB226_13:
	movq 304(%rsp), %rcx
	incq %r12
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/index.rs:272
		&(*slice)[self]
	subq %rdi, %rcx
	addq %r11, %rdi
	movq %rdi, %rbp
	shlq $4, %rbp
	addq %rax, %rbp
	.p2align	4
.LBB226_14:
	cmpq %rsi, %rdi
	jae .LBB226_419
		// crates/impact_voxel/src/object/extraction.rs:684
		if let VoxelChunk::NonUniform(_) = chunk {
	cmpb $3, (%rbp)
	adcq $0, %r13
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:1916
		fn lt(&self, other: &Self) -> bool { *self <  *other }
	incq %rdi
	addq $16, %rbp
	decq %rcx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/iter/range.rs:900
		if self.start < self.end {
	jne .LBB226_14
	incq %rbx
	addq %r8, %r15
	subq %r8, %r14
	addq %r8, %r11
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:1916
		fn lt(&self, other: &Self) -> bool { *self <  *other }
	cmpq (%rsp), %r12
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/iter/range.rs:900
		if self.start < self.end {
	jne .LBB226_8
	movq 264(%rsp), %rcx
	movq 312(%rsp), %rdi
	movq 248(%rsp), %r15
	movq 560(%rsp), %r14
	movq 184(%rsp), %r11
	movq 320(%rsp), %rdx
	incq %rcx
	addq %rdi, %r15
	subq %rdi, %r14
	addq %rdi, %r11
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:1916
		fn lt(&self, other: &Self) -> bool { *self <  *other }
	cmpq 592(%rsp), %rdx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/iter/range.rs:900
		if self.start < self.end {
	jne .LBB226_7
.LBB226_18:
	movb $1, %bpl
	.cfi_escape 0x2e, 0x00
		// crates/impact_alloc/src/arena.rs:53
		THREAD_LOCAL_ARENA_POOL.with(|pool| pool.borrow_mut().acquire())
	vzeroupper
	callq <std::thread::local::LocalKey<core::cell::RefCell<impact_alloc::arena::ArenaPool>>>::with::<<impact_alloc::arena::ArenaPool>::get_arena::{closure#0}, impact_alloc::arena::PoolArena>
	movq 32(%rsp), %rbx
	movq %rax, 64(%rsp)
	movq %rdx, 72(%rsp)
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/allocator-api2-0.2.21/src/stable/raw_vec.rs:253
		if mem::size_of::<T>() == 0 || capacity == 0 {
	testq %rbx, %rbx
	je .LBB226_26
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/alloc/layout.rs:573
		if element_size != 0 && n > Layout::max_size_for_alignment(alignment) / element_size {
	movq %rbx, %rcx
	shrq $58, %rcx
	jne .LBB226_449
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cell.rs:555
		unsafe { *self.value.get() }
	movq 16(%rax), %rcx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/alloc/layout.rs:581
		let array_size = unsafe { unchecked_mul(element_size, n) };
	shlq $5, %rbx
	movq 632(%rsp), %r14
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cell.rs:555
		unsafe { *self.value.get() }
	movq 32(%rcx), %rdi
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mut_ptr.rs:470
		unsafe { intrinsics::arith_offset(self, count) as *mut T }
	andq $-16, %rdi
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:1952
		if aligned_ptr < start || aligned_size > capacity {
	movq %rdi, %rdx
	subq (%rcx), %rdx
	setb %sil
	cmpq %rdx, %rbx
	seta %dl
	orb %sil, %dl
	je .LBB226_27
	.cfi_escape 0x2e, 0x00
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:1883
		self.alloc_layout_slow(layout).ok_or(AllocErr)
	movl $16, %esi
	movq %rax, %rdi
	movq %rbx, %rdx
	callq *<bumpalo::Bump>::alloc_layout_slow@GOTPCREL(%rip)
	movq %rax, 160(%rsp)
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/allocator-api2-0.2.21/src/stable/raw_vec.rs:270
		let ptr = match result {
	testq %rax, %rax
	je .LBB226_432
		// crates/impact_alloc/src/arena.rs:169
		unsafe { &*self.arena_ptr }
	movq 64(%rsp), %rax
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/allocator-api2-0.2.21/src/stable/raw_vec.rs:270
		let ptr = match result {
	jmp .LBB226_28
.LBB226_25:
	movq 384(%rsp), %rax
		// crates/impact_voxel/src/object/extraction.rs:662
		return ExtractionResult::NotExtracted(buffers);
	vmovdqu64 (%rax), %zmm0
	vmovdqu64 112(%rax), %zmm2
	vmovdqu64 64(%rax), %zmm1
	vmovdqu64 %zmm2, 120(%rdi)
	vmovdqu64 %zmm1, 72(%rdi)
	vmovdqu64 %zmm0, 8(%rdi)
	movq $-1, (%rdi)
	jmp .LBB226_415
.LBB226_26:
	movl $16, %eax
	movq $0, 400(%rsp)
	movq $0, 136(%rsp)
	movq $0, 144(%rsp)
	movq %rax, 168(%rsp)
	movl $4, %eax
	movq %rax, 176(%rsp)
	movl $16, %eax
	movq %rax, 160(%rsp)
	jmp .LBB226_79
.LBB226_27:
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mut_ptr.rs:470
		unsafe { intrinsics::arith_offset(self, count) as *mut T }
	subq %rbx, %rdi
	movq %rdi, 160(%rsp)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/mem/mod.rs:931
		crate::intrinsics::write_via_move(dest, src);
	movq %rdi, 32(%rcx)
.LBB226_28:
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cell.rs:555
		unsafe { *self.value.get() }
	movq 16(%rax), %rcx
	movq 32(%rcx), %rdi
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mut_ptr.rs:470
		unsafe { intrinsics::arith_offset(self, count) as *mut T }
	andq $-16, %rdi
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:1952
		if aligned_ptr < start || aligned_size > capacity {
	movq %rdi, %rdx
	subq (%rcx), %rdx
	setb %sil
	cmpq %rdx, %rbx
	seta %dl
	orb %sil, %dl
	je .LBB226_32
	.cfi_escape 0x2e, 0x00
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:1883
		self.alloc_layout_slow(layout).ok_or(AllocErr)
	movl $16, %esi
	movq %rax, %rdi
	movq %rbx, %rdx
	callq *<bumpalo::Bump>::alloc_layout_slow@GOTPCREL(%rip)
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/allocator-api2-0.2.21/src/stable/raw_vec.rs:270
		let ptr = match result {
	testq %rax, %rax
	je .LBB226_433
	movq %rax, 168(%rsp)
		// crates/impact_alloc/src/arena.rs:169
		unsafe { &*self.arena_ptr }
	movq 64(%rsp), %rax
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/allocator-api2-0.2.21/src/stable/raw_vec.rs:270
		let ptr = match result {
	jmp .LBB226_33
.LBB226_32:
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mut_ptr.rs:470
		unsafe { intrinsics::arith_offset(self, count) as *mut T }
	subq %rbx, %rdi
	movq %rdi, 168(%rsp)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/mem/mod.rs:931
		crate::intrinsics::write_via_move(dest, src);
	movq %rdi, 32(%rcx)
.LBB226_33:
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cell.rs:555
		unsafe { *self.value.get() }
	movq 16(%rax), %rcx
	movq 32(%rsp), %rbx
	movq 32(%rcx), %rdi
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/alloc/layout.rs:581
		let array_size = unsafe { unchecked_mul(element_size, n) };
	shlq $4, %rbx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mut_ptr.rs:470
		unsafe { intrinsics::arith_offset(self, count) as *mut T }
	andq $-4, %rdi
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:1952
		if aligned_ptr < start || aligned_size > capacity {
	movq %rdi, %rdx
	subq (%rcx), %rdx
	setb %sil
	cmpq %rdx, %rbx
	seta %dl
	orb %sil, %dl
	je .LBB226_37
	.cfi_escape 0x2e, 0x00
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:1883
		self.alloc_layout_slow(layout).ok_or(AllocErr)
	movl $4, %esi
	movq %rax, %rdi
	movq %rbx, %rdx
	callq *<bumpalo::Bump>::alloc_layout_slow@GOTPCREL(%rip)
	movq %rax, 176(%rsp)
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/allocator-api2-0.2.21/src/stable/raw_vec.rs:270
		let ptr = match result {
	testq %rax, %rax
	jne .LBB226_38
	.cfi_escape 0x2e, 0x00
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/allocator-api2-0.2.21/src/stable/raw_vec.rs:272
		Err(_) => handle_alloc_error(layout),
	movl $4, %edi
	movq %rbx, %rsi
	callq *alloc::alloc::handle_alloc_error@GOTPCREL(%rip)
	jmp .LBB226_452
.LBB226_37:
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mut_ptr.rs:470
		unsafe { intrinsics::arith_offset(self, count) as *mut T }
	subq %rbx, %rdi
	movq %rdi, 176(%rsp)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/mem/mod.rs:931
		crate::intrinsics::write_via_move(dest, src);
	movq %rdi, 32(%rcx)
.LBB226_38:
	movq 32(%rsp), %rcx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mut_ptr.rs:961
		unsafe { intrinsics::offset(self, count) }
	addq %r14, %rbx
	movl $1, %edi
	movq %r14, %rdx
	xorl %r14d, %r14d
	movq $0, 264(%rsp)
	xorl %r15d, %r15d
	xorl %esi, %esi
	movq %rbx, 320(%rsp)
	movq %rcx, 144(%rsp)
	movq %rcx, 136(%rsp)
	jmp .LBB226_41
	.p2align	4
.LBB226_39:
	movq 168(%rsp), %r12
.LBB226_40:
	vmovdqa 48(%rsp), %xmm0
	vmovd 80(%rsp), %xmm1
	movq 632(%rsp), %rdx
	movq 248(%rsp), %rdi
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/non_null.rs:1714
		self.as_ptr() == other.as_ptr()
	addq $-32, 264(%rsp)
	movq %r12, 168(%rsp)
	leaq (%rdx,%r15,8), %rax
	addq $2, %r15
	incq %rdi
	addq $16, %rax
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mod.rs:1933
		intrinsics::write_via_move(dst, src)
	vmovdqa %xmm0, (%r12,%r14)
	vmovd %xmm1, 16(%r12,%r14)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/non_null.rs:1714
		self.as_ptr() == other.as_ptr()
	addq $32, %r14
	cmpq 320(%rsp), %rax
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/iter/macros.rs:180
		if ptr == crate::intrinsics::transmute::<$ptr, NonNull<T>>(end_or_len) {
	je .LBB226_78
.LBB226_41:
		// crates/impact_math/src/vector.rs:1385
		self.y
	vmovsd 4(%rdx,%r15,8), %xmm0
	movq 144(%rsp), %r12
	movq 160(%rsp), %rbx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/non_null.rs:656
		unsafe { transmute(intrinsics::offset(self.as_ptr(), count)) }
	cmpq %r15, %rdi
	movq %r15, %rbp
	movl $4, %eax
	movq %rsi, (%rsp)
	movq %rdi, 248(%rsp)
	cmovaq %rdi, %rbp
	cmpq $5, %rbp
	cmovbq %rax, %rbp
	vmovaps %xmm0, 48(%rsp)
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/glam-0.30.10/src/f32/sse2/vec3a.rs:109
		unsafe { UnionCast { a: [x, y, z, z] }.v }
	vmovss (%rdx,%r15,8), %xmm0
	vmovaps %xmm0, 560(%rsp)
		// crates/impact_geometry/src/plane.rs:320
		Plane::new(self.unit_normal.aligned(), self.displacement)
	vmovss 12(%rdx,%r15,8), %xmm0
	vmovss %xmm0, 80(%rsp)
		// crates/impact_voxel/src/object/extraction.rs:703
		plane.displacement() - INTERIOR_MARGIN,
	vaddss .LCPI226_11(%rip), %xmm0, %xmm0
	vmovss %xmm0, 184(%rsp)
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/allocator-api2-0.2.21/src/stable/vec/mod.rs:1793
		if self.len == self.buf.capacity() {
	cmpq %r12, %rsi
	jne .LBB226_49
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/num/uint_macros.rs:896
		if intrinsics::unlikely(intrinsics::add_with_overflow(self, rhs).1) {
	leaq 1(%r12), %rcx
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/allocator-api2-0.2.21/src/stable/raw_vec.rs:498
		let cap = cmp::max(self.cap * 2, required_cap);
	leaq (%r12,%r12), %rax
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:1038
		if other < self { self } else { other }
	movl $4, %edx
	movq %rbx, 152(%rsp)
	cmpq %rax, %rcx
	cmovaq %rcx, %rax
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/alloc/layout.rs:573
		if element_size != 0 && n > Layout::max_size_for_alignment(alignment) / element_size {
	movabsq $288230376151711743, %rcx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:1038
		if other < self { self } else { other }
	cmpq $5, %rax
	cmovaeq %rax, %rdx
	movq %rdx, 144(%rsp)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/alloc/layout.rs:573
		if element_size != 0 && n > Layout::max_size_for_alignment(alignment) / element_size {
	shlq $5, %rdx
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/allocator-api2-0.2.21/src/stable/raw_vec.rs:333
		if mem::size_of::<T>() == 0 || self.cap == 0 {
	testq %r12, %r12
	je .LBB226_50
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/alloc/layout.rs:573
		if element_size != 0 && n > Layout::max_size_for_alignment(alignment) / element_size {
	cmpq %rcx, %rax
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:966
		match self {
	ja .LBB226_442
		// crates/impact_alloc/src/arena.rs:169
		unsafe { &*self.arena_ptr }
	movq 64(%rsp), %rdi
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/alloc/layout.rs:573
		if element_size != 0 && n > Layout::max_size_for_alignment(alignment) / element_size {
	movq %r12, %rsi
	shlq $5, %rsi
	movq %rsi, 200(%rsp)
	movq 16(%rdi), %rax
	movq 32(%rax), %rcx
	movq %rcx, %r8
	cmpq %rbx, %rcx
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:2355
		if align_is_compatible && self.is_last_allocation(ptr) {
	je .LBB226_57
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:1898
		let start = footer.data.as_ptr();
	movq (%rax), %rcx
.LBB226_46:
	movq %r8, %rbx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mut_ptr.rs:470
		unsafe { intrinsics::arith_offset(self, count) as *mut T }
	andq $-16, %rbx
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:1952
		if aligned_ptr < start || aligned_size > capacity {
	cmpq %rcx, %rbx
	jb .LBB226_54
	movq %rbx, %rsi
	subq %rcx, %rsi
	cmpq %rsi, %rdx
	ja .LBB226_54
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mut_ptr.rs:470
		unsafe { intrinsics::arith_offset(self, count) as *mut T }
	subq %rdx, %rbx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/mem/mod.rs:931
		crate::intrinsics::write_via_move(dest, src);
	movq %rbx, 32(%rax)
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:1880
		if let Some(p) = self.try_alloc_layout_fast(layout) {
	jmp .LBB226_56
	.p2align	4
.LBB226_49:
	movq %r12, 144(%rsp)
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/allocator-api2-0.2.21/src/stable/vec/mod.rs:1793
		if self.len == self.buf.capacity() {
	jmp .LBB226_63
.LBB226_50:
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/alloc/layout.rs:573
		if element_size != 0 && n > Layout::max_size_for_alignment(alignment) / element_size {
	cmpq %rcx, %rax
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:966
		match self {
	ja .LBB226_442
		// crates/impact_alloc/src/arena.rs:169
		unsafe { &*self.arena_ptr }
	movq 64(%rsp), %rdi
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cell.rs:555
		unsafe { *self.value.get() }
	movq 16(%rdi), %rax
	movq 32(%rax), %rcx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mut_ptr.rs:470
		unsafe { intrinsics::arith_offset(self, count) as *mut T }
	andq $-16, %rcx
	movq %rcx, %rbx
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:1952
		if aligned_ptr < start || aligned_size > capacity {
	subq (%rax), %rcx
	jb .LBB226_61
	cmpq %rcx, %rdx
	ja .LBB226_61
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mut_ptr.rs:470
		unsafe { intrinsics::arith_offset(self, count) as *mut T }
	subq %rdx, %rbx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/mem/mod.rs:931
		crate::intrinsics::write_via_move(dest, src);
	movq %rbx, 32(%rax)
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:1880
		if let Some(p) = self.try_alloc_layout_fast(layout) {
	jmp .LBB226_63
.LBB226_54:
	.cfi_escape 0x2e, 0x00
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:1883
		self.alloc_layout_slow(layout).ok_or(AllocErr)
	movl $16, %esi
	movq %r12, 552(%rsp)
	movq %rdx, 312(%rsp)
	callq *<bumpalo::Bump>::alloc_layout_slow@GOTPCREL(%rip)
	movq 312(%rsp), %rdx
	movq %rax, %rbx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/option.rs:1335
		match self {
	testq %rax, %rax
	je .LBB226_446
.LBB226_56:
	.cfi_escape 0x2e, 0x00
	movq 152(%rsp), %rsi
	movq 200(%rsp), %rdx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mod.rs:551
		unsafe { crate::intrinsics::copy_nonoverlapping(src, dst, count) }
	movq %rbx, %rdi
	callq *memcpy@GOTPCREL(%rip)
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:2371
		}
	jmp .LBB226_63
.LBB226_57:
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:2358
		let delta = new_size - old_size;
	movq %rdx, %r9
	subq %rsi, %r9
	movabsq $9223372036854775793, %rcx
	cmpq %rcx, %r9
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/mem/alignment.rs:137
		if align.is_power_of_two() {
	jae .LBB226_446
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:1898
		let start = footer.data.as_ptr();
	movq (%rax), %rcx
	movq 152(%rsp), %rsi
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:453
		ptr.wrapping_sub(ptr as usize & (divisor - 1))
	andl $15, %esi
	movq %r8, %rbx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mut_ptr.rs:470
		unsafe { intrinsics::arith_offset(self, count) as *mut T }
	subq %rsi, %rbx
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:1952
		if aligned_ptr < start || aligned_size > capacity {
	movq %rbx, %rsi
	subq %rcx, %rsi
	jb .LBB226_46
	cmpq %rsi, %r9
	ja .LBB226_46
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mut_ptr.rs:470
		unsafe { intrinsics::arith_offset(self, count) as *mut T }
	subq %r9, %rbx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/mem/mod.rs:931
		crate::intrinsics::write_via_move(dest, src);
	movq %rbx, 32(%rax)
	.cfi_escape 0x2e, 0x00
	movq 152(%rsp), %rsi
	movq 200(%rsp), %rdx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mod.rs:642
		crate::intrinsics::copy(src, dst, count)
	movq %rbx, %rdi
	callq *memmove@GOTPCREL(%rip)
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:2371
		}
	jmp .LBB226_63
.LBB226_61:
	.cfi_escape 0x2e, 0x00
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:1883
		self.alloc_layout_slow(layout).ok_or(AllocErr)
	movl $16, %esi
	movq $0, 552(%rsp)
	movq %rdx, 312(%rsp)
	callq *<bumpalo::Bump>::alloc_layout_slow@GOTPCREL(%rip)
	movq 312(%rsp), %rdx
	movq %rax, %rbx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:966
		match self {
	testq %rax, %rax
	je .LBB226_446
	.p2align	4
.LBB226_63:
	vpmovsxbd .LCPI226_22(%rip), %xmm0
	vmovaps 48(%rsp), %xmm1
	movq (%rsp), %rax
	movq %rbx, 160(%rsp)
		// crates/impact_voxel/src/object/extraction.rs:707
		plane.displacement() + EXTERIOR_MARGIN,
	leaq 1(%rax), %rsi
	vpermt2ps 560(%rsp), %xmm0, %xmm1
	vmovss 184(%rsp), %xmm0
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mod.rs:1933
		intrinsics::write_via_move(dst, src)
	vmovaps %xmm1, (%rbx,%r14)
	vmovss %xmm0, 16(%rbx,%r14)
	vmovss 80(%rsp), %xmm0
	vmovaps %xmm1, 48(%rsp)
		// crates/impact_voxel/src/object/extraction.rs:707
		plane.displacement() + EXTERIOR_MARGIN,
	vaddss .LCPI226_1(%rip), %xmm0, %xmm0
	vmovss %xmm0, 80(%rsp)
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/allocator-api2-0.2.21/src/stable/vec/mod.rs:1793
		if self.len == self.buf.capacity() {
	cmpq 136(%rsp), %rax
	jne .LBB226_39
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:1038
		if other < self { self } else { other }
	cmpq %r15, %rsi
	movq %r15, %rax
	movl $4, %ebx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/alloc/layout.rs:573
		if element_size != 0 && n > Layout::max_size_for_alignment(alignment) / element_size {
	movabsq $288230376151711743, %rcx
	movq %rsi, 400(%rsp)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:1038
		if other < self { self } else { other }
	cmovaq %rsi, %rax
	cmpq $5, %rax
	cmovaeq %rax, %rbx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/alloc/layout.rs:573
		if element_size != 0 && n > Layout::max_size_for_alignment(alignment) / element_size {
	cmpq %rcx, %rax
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:966
		match self {
	ja .LBB226_443
		// crates/impact_alloc/src/arena.rs:169
		unsafe { &*self.arena_ptr }
	movq 64(%rsp), %rdi
	movq %rbx, 136(%rsp)
	shlq $5, %rbx
	movq 16(%rdi), %rax
	movq 32(%rax), %r12
	cmpq 168(%rsp), %r12
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:2355
		if align_is_compatible && self.is_last_allocation(ptr) {
	je .LBB226_74
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:1898
		let start = footer.data.as_ptr();
	movq (%rax), %rcx
.LBB226_67:
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mut_ptr.rs:470
		unsafe { intrinsics::arith_offset(self, count) as *mut T }
	andq $-16, %r12
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:1952
		if aligned_ptr < start || aligned_size > capacity {
	cmpq %rcx, %r12
	jb .LBB226_70
	movq %r12, %rdx
	subq %rcx, %rdx
	cmpq %rdx, %rbx
	ja .LBB226_70
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mut_ptr.rs:470
		unsafe { intrinsics::arith_offset(self, count) as *mut T }
	subq %rbx, %r12
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/mem/mod.rs:931
		crate::intrinsics::write_via_move(dest, src);
	movq %r12, 32(%rax)
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:1880
		if let Some(p) = self.try_alloc_layout_fast(layout) {
	jmp .LBB226_72
	.p2align	4
.LBB226_70:
	.cfi_escape 0x2e, 0x00
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:1883
		self.alloc_layout_slow(layout).ok_or(AllocErr)
	movl $16, %esi
	movq %rbx, %rdx
	callq *<bumpalo::Bump>::alloc_layout_slow@GOTPCREL(%rip)
	movq %rax, %r12
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/option.rs:1335
		match self {
	testq %rax, %rax
	je .LBB226_444
.LBB226_72:
	.cfi_escape 0x2e, 0x00
	movq 168(%rsp), %rsi
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mod.rs:551
		unsafe { crate::intrinsics::copy_nonoverlapping(src, dst, count) }
	movq %r12, %rdi
	movq %r14, %rdx
	callq *memcpy@GOTPCREL(%rip)
.LBB226_73:
	movq 400(%rsp), %rsi
	jmp .LBB226_40
.LBB226_74:
	movq 264(%rsp), %rcx
	shlq $5, %rbp
	movabsq $9223372036854775793, %rdx
	leaq (%rcx,%rbp), %rsi
	cmpq %rdx, %rsi
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/mem/alignment.rs:137
		if align.is_power_of_two() {
	jae .LBB226_444
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:1898
		let start = footer.data.as_ptr();
	movq (%rax), %rcx
	movq 168(%rsp), %r8
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:453
		ptr.wrapping_sub(ptr as usize & (divisor - 1))
	andl $15, %r8d
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mut_ptr.rs:470
		unsafe { intrinsics::arith_offset(self, count) as *mut T }
	movq %r12, %rdx
	subq %r8, %rdx
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:1952
		if aligned_ptr < start || aligned_size > capacity {
	movq %rdx, %r8
	subq %rcx, %r8
	jb .LBB226_67
	cmpq %r8, %rsi
	ja .LBB226_67
	negq %rbp
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/non_null.rs:238
		transmute(ptr)
	leaq (%r14,%rbp), %r12
	addq %rdx, %rbp
	addq %rdx, %r12
	addq %r14, %rbp
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/mem/mod.rs:931
		crate::intrinsics::write_via_move(dest, src);
	movq %r12, 32(%rax)
	.cfi_escape 0x2e, 0x00
	movq 168(%rsp), %rsi
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mod.rs:642
		crate::intrinsics::copy(src, dst, count)
	movq %rbp, %rdi
	movq %r14, %rdx
	callq *memmove@GOTPCREL(%rip)
	jmp .LBB226_73
.LBB226_78:
	movq %rsi, 400(%rsp)
.LBB226_79:
	movq 384(%rsp), %rbx
	movq 88(%rsp), %r14
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/vec/mod.rs:3035
		self.len = 0;
	movq $0, 16(%rbx)
	movq $0, 40(%rbx)
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hashbrown-0.16.1/src/raw/mod.rs:873
		if self.is_empty() {
	cmpq $0, 168(%rbx)
	movq 152(%rbx), %rax
	movq %rax, 584(%rsp)
	je .LBB226_84
	movq 584(%rsp), %r15
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hashbrown-0.16.1/src/raw/mod.rs:3208
		if !self.is_empty_singleton() {
	testq %r15, %r15
	je .LBB226_82
	movq 384(%rsp), %rbx
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hashbrown-0.16.1/src/raw/mod.rs:2692
		self.bucket_mask + 1 + Group::WIDTH
	leaq 17(%r15), %rdx
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hashbrown-0.16.1/src/raw/mod.rs:2671
		unsafe { slice::from_raw_parts_mut(self.ctrl.as_ptr().cast(), self.num_ctrl_bytes()) }
	movq 144(%rbx), %rdi
	.cfi_escape 0x2e, 0x00
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mod.rs:713
		crate::intrinsics::write_bytes(dst, val, count)
	movl $255, %esi
	callq *memset@GOTPCREL(%rip)
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hashbrown-0.16.1/src/raw/mod.rs:185
		if bucket_mask < 8 {
	leaq 1(%r15), %rax
	movq %rax, %rcx
	shrq $3, %rcx
	andq $-8, %rax
	subq %rcx, %rax
	cmpq $8, %r15
	cmovbq %r15, %rax
	jmp .LBB226_83
.LBB226_82:
	movq 384(%rsp), %rbx
	xorl %eax, %eax
.LBB226_83:
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hashbrown-0.16.1/src/raw/mod.rs:3211
		self.items = 0;
	movq $0, 168(%rbx)
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hashbrown-0.16.1/src/raw/mod.rs:3212
		self.growth_left = bucket_mask_to_capacity(self.bucket_mask);
	movq %rax, 160(%rbx)
.LBB226_84:
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/vec/mod.rs:3035
		self.len = 0;
	movq $0, 64(%rbx)
	movq $0, 88(%rbx)
	movq $0, 112(%rbx)
	movq $0, 136(%rbx)
		// crates/impact_voxel/src/object/extraction.rs:715
		invalidated_mesh_chunk_indices: poly_invalidated_mesh_chunk_indices,
	movq 144(%rbx), %rdx
	movq 160(%rbx), %rcx
		// crates/impact_voxel/src/object/extraction.rs:713
		chunks: mut poly_chunks,
	vmovups (%rbx), %xmm0
		// crates/impact_voxel/src/object/extraction.rs:714
		voxels: mut poly_voxels,
	movq 40(%rbx), %rsi
		// crates/impact_voxel/src/object/extraction.rs:713
		chunks: mut poly_chunks,
	movq 16(%rbx), %rax
	movq %rdx, 800(%rsp)
		// crates/impact_voxel/src/object/extraction.rs:715
		invalidated_mesh_chunk_indices: poly_invalidated_mesh_chunk_indices,
	movq %rcx, 1296(%rsp)
	movq 168(%rbx), %rdx
		// crates/impact_voxel/src/object/extraction.rs:716
		split_detector_buffers: poly_split_detector_buffers,
	movq 48(%rbx), %rcx
		// crates/impact_voxel/src/object/extraction.rs:713
		chunks: mut poly_chunks,
	movq %rax, 112(%rsp)
		// crates/impact_voxel/src/object/extraction.rs:714
		voxels: mut poly_voxels,
	movq %rsi, 368(%rsp)
		// crates/impact_voxel/src/object/extraction.rs:713
		chunks: mut poly_chunks,
	vmovaps %xmm0, 96(%rsp)
		// crates/impact_voxel/src/object/extraction.rs:714
		voxels: mut poly_voxels,
	vmovdqu 24(%rbx), %xmm0
		// crates/impact_voxel/src/object/extraction.rs:715
		invalidated_mesh_chunk_indices: poly_invalidated_mesh_chunk_indices,
	movq %rdx, 1304(%rsp)
	movq %rcx, 840(%rsp)
		// crates/impact_voxel/src/object/extraction.rs:716
		split_detector_buffers: poly_split_detector_buffers,
	movq 56(%rbx), %rdx
	movq 72(%rbx), %rcx
	movq %rdx, 808(%rsp)
	movq %rcx, 848(%rsp)
	movq 80(%rbx), %rdx
	movq 96(%rbx), %rcx
		// crates/impact_voxel/src/object/extraction.rs:714
		voxels: mut poly_voxels,
	vmovdqa %xmm0, 352(%rsp)
	movq %rdx, 816(%rsp)
	movq %rcx, 856(%rsp)
		// crates/impact_voxel/src/object/extraction.rs:716
		split_detector_buffers: poly_split_detector_buffers,
	movq 104(%rbx), %rdx
	movq 120(%rbx), %rcx
	movq %rdx, 824(%rsp)
	movq %rcx, 864(%rsp)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/raw_vec/mod.rs:619
		if elem_size == 0 { usize::MAX } else { self.cap.as_inner() }
	movq 352(%rsp), %rcx
		// crates/impact_voxel/src/object/extraction.rs:716
		split_detector_buffers: poly_split_detector_buffers,
	movq 128(%rbx), %rdx
	movq %rdx, 832(%rsp)
		// crates/impact_voxel/src/object/extraction.rs:719
		poly_voxels.reserve(touched_non_uniform_chunk_count * CHUNK_VOXEL_COUNT);
	movq %r13, %rdx
	shlq $12, %rdx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/num/uint_macros.rs:2621
		intrinsics::wrapping_sub(self, rhs)
	subq %rsi, %rcx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/raw_vec/mod.rs:767
		additional > self.capacity(elem_layout.size()).wrapping_sub(len)
	cmpq %rcx, %rdx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/raw_vec/mod.rs:673
		if self.needs_to_grow(len, additional, elem_layout) {
	ja .LBB226_427
.LBB226_85:
	vmovdqu 1344(%rsp), %ymm1
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/vec/mod.rs:1471
		self.buf.reserve(self.len, additional);
	movq 112(%rsp), %rsi
	vpmaxuq 1312(%rsp), %xmm1, %xmm0
	vpsubq %xmm1, %xmm0, %xmm0
	vmovq %xmm0, %rax
	vpextrq $1, %xmm0, %rdx
	vmovdqa %xmm0, 1872(%rsp)
	imulq %rax, %rdx
	imulq 872(%rsp), %rdx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/raw_vec/mod.rs:619
		if elem_size == 0 { usize::MAX } else { self.cap.as_inner() }
	movq 96(%rsp), %rax
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/num/uint_macros.rs:2621
		intrinsics::wrapping_sub(self, rhs)
	subq %rsi, %rax
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/raw_vec/mod.rs:767
		additional > self.capacity(elem_layout.size()).wrapping_sub(len)
	cmpq %rax, %rdx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/raw_vec/mod.rs:673
		if self.needs_to_grow(len, additional, elem_layout) {
	ja .LBB226_428
.LBB226_86:
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/allocator-api2-0.2.21/src/stable/raw_vec.rs:253
		if mem::size_of::<T>() == 0 || capacity == 0 {
	testq %r13, %r13
	je .LBB226_91
	movabsq $384307168202282325, %rax
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/alloc/layout.rs:573
		if element_size != 0 && n > Layout::max_size_for_alignment(alignment) / element_size {
	cmpq %rax, %r13
	ja .LBB226_450
		// crates/impact_alloc/src/arena.rs:169
		unsafe { &*self.arena_ptr }
	movq 64(%rsp), %rdi
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/alloc/layout.rs:581
		let array_size = unsafe { unchecked_mul(element_size, n) };
	leaq (,%r13,8), %rax
	leaq (%rax,%rax,2), %rbx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cell.rs:555
		unsafe { *self.value.get() }
	movq 16(%rdi), %rax
	movq 32(%rax), %rcx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mut_ptr.rs:470
		unsafe { intrinsics::arith_offset(self, count) as *mut T }
	andq $-8, %rcx
	movq %rcx, %rdx
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:1952
		if aligned_ptr < start || aligned_size > capacity {
	subq (%rax), %rcx
	jb .LBB226_92
	cmpq %rcx, %rbx
	ja .LBB226_92
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mut_ptr.rs:470
		unsafe { intrinsics::arith_offset(self, count) as *mut T }
	subq %rbx, %rdx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/mem/mod.rs:931
		crate::intrinsics::write_via_move(dest, src);
	movq %rdx, 32(%rax)
	movq %rdx, %rax
	jmp .LBB226_94
.LBB226_91:
	movl $8, %eax
	movq %rax, 24(%rsp)
	movq %rax, 128(%rsp)
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/allocator-api2-0.2.21/src/stable/raw_vec.rs:253
		if mem::size_of::<T>() == 0 || capacity == 0 {
	jmp .LBB226_100
.LBB226_92:
	.cfi_escape 0x2e, 0x00
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:1883
		self.alloc_layout_slow(layout).ok_or(AllocErr)
	movl $8, %esi
	movq %rbx, %rdx
	vzeroupper
	callq *<bumpalo::Bump>::alloc_layout_slow@GOTPCREL(%rip)
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/allocator-api2-0.2.21/src/stable/raw_vec.rs:270
		let ptr = match result {
	testq %rax, %rax
	je .LBB226_434
.LBB226_94:
	movabsq $164703072086692425, %rcx
	movq %rax, 24(%rsp)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/alloc/layout.rs:573
		if element_size != 0 && n > Layout::max_size_for_alignment(alignment) / element_size {
	cmpq %rcx, %r13
	ja .LBB226_451
		// crates/impact_alloc/src/arena.rs:169
		unsafe { &*self.arena_ptr }
	movq 64(%rsp), %rdi
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/alloc/layout.rs:581
		let array_size = unsafe { unchecked_mul(element_size, n) };
	imulq $56, %r13, %rbx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cell.rs:555
		unsafe { *self.value.get() }
	movq 16(%rdi), %rax
	movq 32(%rax), %rcx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mut_ptr.rs:470
		unsafe { intrinsics::arith_offset(self, count) as *mut T }
	andq $-8, %rcx
	movq %rcx, %rdx
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:1952
		if aligned_ptr < start || aligned_size > capacity {
	subq (%rax), %rcx
	jb .LBB226_98
	cmpq %rcx, %rbx
	ja .LBB226_98
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mut_ptr.rs:470
		unsafe { intrinsics::arith_offset(self, count) as *mut T }
	subq %rbx, %rdx
	movq %rdx, 128(%rsp)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/mem/mod.rs:931
		crate::intrinsics::write_via_move(dest, src);
	movq %rdx, 32(%rax)
	jmp .LBB226_100
.LBB226_98:
	.cfi_escape 0x2e, 0x00
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:1883
		self.alloc_layout_slow(layout).ok_or(AllocErr)
	movl $8, %esi
	movq %rbx, %rdx
	vzeroupper
	callq *<bumpalo::Bump>::alloc_layout_slow@GOTPCREL(%rip)
	movq %rax, 128(%rsp)
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/allocator-api2-0.2.21/src/stable/raw_vec.rs:270
		let ptr = match result {
	testq %rax, %rax
	je .LBB226_435
.LBB226_100:
	.cfi_escape 0x2e, 0x00
	leaq 64(%rsp), %rbx
	leaq 1376(%rsp), %rdi
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hashbrown-0.16.1/src/raw/mod.rs:1699
		match Self::fallible_with_capacity(alloc, table_layout, capacity, Fallibility::Infallible) {
	movl $1, %ecx
	movq %r13, %rdx
	movq %rbx, %rsi
	vzeroupper
	callq <hashbrown::raw::RawTableInner>::fallible_with_capacity::<&impact_alloc::arena::PoolArena>
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hashbrown-0.16.1/src/map.rs:579
		Self {
	vmovdqu 1376(%rsp), %ymm0
	movq 408(%rsp), %rax
	vmovdqu %ymm0, 1120(%rsp)
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hashbrown-0.16.1/src/set.rs:596
		Self {
	movq %rbx, 1152(%rsp)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:1916
		fn lt(&self, other: &Self) -> bool { *self <  *other }
	cmpq 592(%rsp), %rax
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/iter/range.rs:900
		if self.start < self.end {
	jae .LBB226_363
	vmovdqu 1312(%rsp), %ymm0
	xorl %esi, %esi
	vpextrq $1, %xmm0, %rax
	vmovdqu 1344(%rsp), %ymm0
	movq %rax, 1192(%rsp)
	vpextrq $1, %xmm0, %rdi
	cmpq %rax, %rdi
	jae .LBB226_364
	movq 304(%rsp), %rax
	cmpq %rax, 296(%rsp)
	jae .LBB226_364
	movq 2528(%rsp), %rax
	movq 400(%rsp), %rdx
	movq 160(%rsp), %rcx
	xorl %ebx, %ebx
	movq %r13, (%rsp)
	xorl %esi, %esi
	movq %rdi, 1176(%rsp)
	vmovss 40(%rax), %xmm0
	vbroadcastss 32(%rax), %xmm1
	shlq $5, %rdx
	addq %rdx, %rcx
	movq %rdx, 1240(%rsp)
	xorl %edx, %edx
	movq %rcx, 1248(%rsp)
	leaq 24(%r14), %rcx
	movq %rcx, 1168(%rsp)
	leaq 48(%r14), %rcx
	movq %rcx, 600(%rsp)
	leaq 152(%r14), %rcx
	movq %rcx, 280(%rsp)
	movq 24(%rax), %rcx
	vmovss %xmm0, 312(%rsp)
	vmovss 36(%rax), %xmm0
	vmulss .LCPI226_12(%rip), %xmm0, %xmm2
	vmulss .LCPI226_13(%rip), %xmm0, %xmm0
	vmovaps %xmm1, 560(%rsp)
	movq %rcx, 248(%rsp)
	movq 16(%rax), %rcx
	movq %rcx, 264(%rsp)
	movq (%rax), %rcx
	movq 8(%rax), %rax
	vmovss %xmm0, 716(%rsp)
	vmulss .LCPI226_14(%rip), %xmm1, %xmm0
	vmulss .LCPI226_15(%rip), %xmm1, %xmm1
	vmovss %xmm2, 552(%rsp)
	movq %rax, 152(%rsp)
	movabsq $576460752303423488, %rax
	movq %rcx, 320(%rsp)
	xorl %ecx, %ecx
	decq %rax
	movq %rax, 888(%rsp)
	movq 32(%rsp), %rax
	movq %rax, 40(%rsp)
	movl %eax, 332(%rsp)
	vmovss %xmm0, 712(%rsp)
	vmulss %xmm1, %xmm1, %xmm0
	vmovaps %xmm1, 1936(%rsp)
	vmulss %xmm0, %xmm1, %xmm2
	vmovss %xmm2, 708(%rsp)
	vmulss .LCPI226_12(%rip), %xmm0, %xmm2
	vmulss .LCPI226_13(%rip), %xmm0, %xmm0
	vmovss %xmm2, 704(%rsp)
	vmulss .LCPI226_14(%rip), %xmm1, %xmm2
	vbroadcastss %xmm1, %xmm1
	vmovss %xmm0, 700(%rsp)
	vmovaps %xmm1, 1920(%rsp)
	vmovss %xmm2, 696(%rsp)
.LBB226_105:
	movq 408(%rsp), %rax
	movq %rcx, 272(%rsp)
	movq %rbx, 536(%rsp)
	movq %rsi, 392(%rsp)
	movq %rdx, 432(%rsp)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/num/uint_macros.rs:971
		intrinsics::unchecked_add(self, rhs)
	leaq 1(%rax), %rcx
	shlq $4, %rax
	vcvtusi2ss %rax, %xmm15, %xmm0
	addq $16, %rax
	movq %rcx, 1184(%rsp)
	vmovaps %xmm0, 1904(%rsp)
	vcvtusi2ss %rax, %xmm15, %xmm0
	vmovaps %xmm0, 1888(%rsp)
	jmp .LBB226_108
	.p2align	4
.LBB226_106:
	movq 16(%rsp), %rax
	movq 8(%rsp), %r13
	movq 88(%rsp), %r14
	movq %rax, (%rsp)
.LBB226_107:
	movq 1200(%rsp), %rdi
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:1916
		fn lt(&self, other: &Self) -> bool { *self <  *other }
	cmpq 1192(%rsp), %rdi
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/iter/range.rs:900
		if self.start < self.end {
	je .LBB226_362
.LBB226_108:
	movq %rdi, %rcx
	shlq $4, %rcx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/num/uint_macros.rs:971
		intrinsics::unchecked_add(self, rhs)
	leaq 1(%rdi), %rax
	movq %rdi, 896(%rsp)
	vcvtusi2ss %rcx, %xmm15, %xmm1
	vmovaps 1904(%rsp), %xmm0
	addq $16, %rcx
	movq %rax, 1200(%rsp)
	vmovaps %xmm1, 2000(%rsp)
	vinsertps $16, %xmm1, %xmm0, %xmm0
	vcvtusi2ss %rcx, %xmm15, %xmm1
	vmovaps %xmm0, 1968(%rsp)
	vmovaps 1888(%rsp), %xmm0
	movq 296(%rsp), %rcx
	vmovaps %xmm1, 1984(%rsp)
	vinsertps $16, %xmm1, %xmm0, %xmm0
	vmovaps %xmm0, 1952(%rsp)
	jmp .LBB226_110
	.p2align	4
.LBB226_109:
	movq 904(%rsp), %rcx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:1916
		fn lt(&self, other: &Self) -> bool { *self <  *other }
	cmpq 304(%rsp), %rcx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/iter/range.rs:900
		if self.start < self.end {
	jae .LBB226_107
.LBB226_110:
	movq 208(%r14), %rdx
	movq 216(%r14), %rbx
	movq (%rsp), %rax
	movq 8(%r14), %r15
	movq %r13, 8(%rsp)
	movq %rdx, 192(%rsp)
	movq %rbx, 784(%rsp)
	imulq 408(%rsp), %rdx
	imulq 896(%rsp), %rbx
	movq %rax, 16(%rsp)
	movq %rcx, %rax
	addq %rdx, %rbx
	movq 16(%r14), %rdx
	movq %rdx, 48(%rsp)
	jmp .LBB226_112
	.p2align	4
.LBB226_111:
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/raw_vec/mod.rs:614
		self.ptr.cast().as_non_null_ptr()
	movq 104(%rsp), %rax
	movq 48(%rsp), %rdx
	movq 904(%rsp), %rsi
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mut_ptr.rs:961
		unsafe { intrinsics::offset(self, count) }
	movq %r14, %rcx
	shlq $4, %rcx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/vec/mod.rs:1046
		self.len = len + 1;
	incq %r14
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mod.rs:1933
		intrinsics::write_via_move(dst, src)
	movb $3, 9(%rax,%rcx)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/vec/mod.rs:1046
		self.len = len + 1;
	movq %r14, 112(%rsp)
	movq %rsi, %rax
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:1916
		fn lt(&self, other: &Self) -> bool { *self <  *other }
	cmpq 304(%rsp), %rsi
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/iter/range.rs:900
		if self.start < self.end {
	jae .LBB226_106
.LBB226_112:
	movq 408(%rsp), %rcx
	movq 896(%rsp), %rsi
	leaq (%rbx,%rax), %rdi
		// crates/impact_voxel/src/object/extraction.rs:741
		let chunk_indices = [chunk_i, chunk_j, chunk_k];
	movq %rcx, 208(%rsp)
	movq %rsi, 216(%rsp)
	movq %rax, 224(%rsp)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/index.rs:272
		&(*slice)[self]
	cmpq %rdx, %rdi
	jae .LBB226_437
	leaq 1(%rax), %rcx
	movq %rdi, 288(%rsp)
	shlq $4, %rdi
		// crates/impact_voxel/src/object/extraction.rs:746
		if let VoxelChunk::Void = chunk {
	movl $2, %esi
	movq %rcx, 904(%rsp)
		// crates/impact_voxel/src/object/extraction.rs:744
		let chunk = self.chunks[chunk_idx];
	movl (%r15,%rdi), %ecx
	movzbl 9(%r15,%rdi), %edx
	movl 4(%r15,%rdi), %r12d
	movq %rcx, 80(%rsp)
	movzbl 8(%r15,%rdi), %ecx
		// crates/impact_voxel/src/object/extraction.rs:746
		if let VoxelChunk::Void = chunk {
	movl %edx, %r8d
	movb %dl, 184(%rsp)
	subb $3, %r8b
	movzbl %r8b, %edx
	cmovbl %esi, %edx
	movl %edx, (%rsp)
	movb %cl, 200(%rsp)
		// crates/impact_voxel/src/object/extraction.rs:744
		let chunk = self.chunks[chunk_idx];
	movzwl 14(%r15,%rdi), %ecx
	movw %cx, 420(%rsp)
	movl 10(%r15,%rdi), %ecx
	movl %ecx, 416(%rsp)
		// crates/impact_voxel/src/object/extraction.rs:746
		if let VoxelChunk::Void = chunk {
	testb %dl, %dl
	je .LBB226_119
		// crates/impact_voxel/src/object.rs:873
		(chunk_k * CHUNK_SIZE) as f32,
	shlq $4, %rax
	vmovaps 1968(%rsp), %xmm2
	vmovaps 1952(%rsp), %xmm3
	leaq (%r15,%rdi), %rcx
	movq %rdi, 472(%rsp)
	vcvtusi2ss %rax, %xmm15, %xmm0
		// crates/impact_voxel/src/object.rs:878
		((chunk_k + 1) * CHUNK_SIZE) as f32,
	addq $16, %rax
	movq %rcx, 256(%rsp)
	movq 168(%rsp), %rcx
	vcvtusi2ss %rax, %xmm15, %xmm1
	movq 1240(%rsp), %rax
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/glam-0.30.10/src/f32/sse2/vec3a.rs:109
		unsafe { UnionCast { a: [x, y, z, z] }.v }
	vshufps $4, %xmm0, %xmm2, %xmm2
	vshufps $4, %xmm1, %xmm3, %xmm3
		// crates/impact_geometry/src/axis_aligned_box.rs:42
		Self {
	vmovaps %xmm2, 2080(%rsp)
	vmovaps %xmm3, 2096(%rsp)
	.p2align	4
.LBB226_115:
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/non_null.rs:1714
		self.as_ptr() == other.as_ptr()
	testq %rax, %rax
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/iter/macros.rs:180
		if ptr == crate::intrinsics::transmute::<$ptr, NonNull<T>>(end_or_len) {
	je .LBB226_121
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/iter/macros.rs:332
		if f(x) {
	vmovaps (%rcx), %xmm2
	vmovaps 2000(%rsp), %xmm4
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/../../stdarch/crates/core_arch/src/x86/sse.rs:1130
		simd_bitmask::<i32x4, u8>(mask) as i32
	vmovmskps %xmm2, %edx
		// crates/impact_geometry/src/axis_aligned_box.rs:648
		(x & 0b010) | ((x & 0b001) << 2) | ((x & 0b100) >> 2)
	vmovd %edx, %xmm3
	vgf2p8affineqb $0, .LCPI226_23(%rip){1to2}, %xmm3, %xmm3
	vmovd %xmm3, %edx
	vmovaps 1984(%rsp), %xmm3
	shrb $5, %dl
		// crates/impact_geometry/src/axis_aligned_box.rs:506
		negative_bitmask_xyz as usize
	movzbl %dl, %edx
		// crates/impact_geometry/src/axis_aligned_box.rs:164
		let is_lower_y = (corner_idx >> 1) & 0b001 == 0;
	testb $2, %dl
	sete %sil
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/iter/macros.rs:332
		if f(x) {
	addq $-32, %rax
		// crates/impact_math/src/point.rs:312
		self.inner.y
	kmovd %esi, %k1
	vmovss %xmm4, %xmm3, %xmm3 {%k1}
		// crates/impact_math/src/point.rs:318
		self.inner.z
	kmovd %edx, %k1
		// crates/impact_geometry/src/axis_aligned_box.rs:168
		if is_lower_x {
	andl $4, %edx
		// crates/impact_math/src/point.rs:318
		self.inner.z
	vmovaps %xmm0, %xmm4
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/glam-0.30.10/src/f32/sse2/vec3a.rs:109
		unsafe { UnionCast { a: [x, y, z, z] }.v }
	vmovss 2080(%rsp,%rdx,4), %xmm5
		// crates/impact_math/src/point.rs:318
		self.inner.z
	vmovss %xmm1, %xmm4, %xmm4 {%k1}
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/glam-0.30.10/src/f32/sse2/vec3a.rs:109
		unsafe { UnionCast { a: [x, y, z, z] }.v }
	vinsertps $16, %xmm3, %xmm5, %xmm3
	vinsertps $32, %xmm4, %xmm3, %xmm3
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/../../stdarch/crates/core_arch/src/x86/sse.rs:88
		unsafe { simd_mul(a, b) }
	vmulps %xmm3, %xmm2, %xmm2
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/../../stdarch/crates/core_arch/src/x86/sse.rs:23
		unsafe { simd_insert!(a, 0, _mm_cvtss_f32(a) + _mm_cvtss_f32(b)) }
	vmovshdup %xmm2, %xmm3
	vaddss %xmm3, %xmm2, %xmm3
	vshufpd $1, %xmm2, %xmm2, %xmm2
	vaddss %xmm3, %xmm2, %xmm2
		// crates/impact_geometry/src/plane.rs:131
		self.compute_signed_distance(point) > 0.0
	vucomiss 16(%rcx), %xmm2
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/non_null.rs:656
		unsafe { transmute(intrinsics::offset(self.as_ptr(), count)) }
	leaq 32(%rcx), %rcx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/iter/macros.rs:332
		if f(x) {
	jbe .LBB226_115
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/vec/mod.rs:1037
		let len = self.len;
	movq 112(%rsp), %r14
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/vec/mod.rs:1040
		if len == self.buf.capacity() {
	cmpq 96(%rsp), %r14
	jne .LBB226_111
	.cfi_escape 0x2e, 0x00
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/vec/mod.rs:1041
		self.buf.grow_one();
	leaq 96(%rsp), %rdi
	vzeroupper
	callq *<alloc::raw_vec::RawVec<impact_voxel::voxel_types::FixedVoxelMaterialProperties>>::grow_one@GOTPCREL(%rip)
	jmp .LBB226_111
	.p2align	4
.LBB226_119:
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/vec/mod.rs:1037
		let len = self.len;
	movq 112(%rsp), %r14
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/vec/mod.rs:1040
		if len == self.buf.capacity() {
	cmpq 96(%rsp), %r14
	jne .LBB226_111
	.cfi_escape 0x2e, 0x00
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/vec/mod.rs:1041
		self.buf.grow_one();
	leaq 96(%rsp), %rdi
	vzeroupper
	callq *<alloc::raw_vec::RawVec<impact_voxel::voxel_types::FixedVoxelMaterialProperties>>::grow_one@GOTPCREL(%rip)
	jmp .LBB226_111
	.p2align	4
.LBB226_121:
	movq 256(%rsp), %rax
	cmpq $0, 400(%rsp)
	leaq 9(%rax), %rax
	movq %rax, 544(%rsp)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/iter/macros.rs:180
		if ptr == crate::intrinsics::transmute::<$ptr, NonNull<T>>(end_or_len) {
	je .LBB226_237
	movq %r12, 688(%rsp)
	movq 632(%rsp), %r12
	movq 160(%rsp), %r15
	xorl %ebx, %ebx
	movq $0, 480(%rsp)
	jmp .LBB226_126
	.p2align	4
.LBB226_123:
	movq 176(%rsp), %r13
.LBB226_124:
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mod.rs:1933
		intrinsics::write_via_move(dst, src)
	vmovups (%r12), %xmm0
	movq 480(%rsp), %rcx
	movq %rbp, 40(%rsp)
	movq %r13, 176(%rsp)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mut_ptr.rs:961
		unsafe { intrinsics::offset(self, count) }
	movq %rcx, %rax
	shlq $4, %rax
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/allocator-api2-0.2.21/src/stable/vec/mod.rs:1799
		self.len += 1;
	incq %rcx
	movq %rcx, 480(%rsp)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mod.rs:1933
		intrinsics::write_via_move(dst, src)
	vmovups %xmm0, (%r13,%rax)
.LBB226_125:
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/non_null.rs:1714
		self.as_ptr() == other.as_ptr()
	addq $16, %r12
	addq $32, %r15
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/iter/adapters/enumerate.rs:82
		self.count += 1;
	incq %rbx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/non_null.rs:1714
		self.as_ptr() == other.as_ptr()
	cmpq 1248(%rsp), %r15
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/iter/macros.rs:180
		if ptr == crate::intrinsics::transmute::<$ptr, NonNull<T>>(end_or_len) {
	je .LBB226_150
.LBB226_126:
		// crates/impact_voxel/src/object/extraction.rs:766
		for (plane_idx, inner_plane) in inner_planes.iter().enumerate() {
	testq %r15, %r15
	je .LBB226_150
		// crates/impact_voxel/src/object/extraction.rs:767
		if !chunk_aabb.lies_in_negative_halfspace_of_plane(inner_plane) {
	vmovaps (%r15), %xmm0
		// crates/impact_math/src/point.rs:312
		self.inner.y
	vmovsd 2100(%rsp), %xmm3
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/glam-0.30.10/src/f32/sse2/vec3a.rs:109
		unsafe { UnionCast { a: [x, y, z, z] }.v }
	vpmovsxbd .LCPI226_24(%rip), %xmm4
		// crates/impact_voxel/src/object/extraction.rs:767
		if !chunk_aabb.lies_in_negative_halfspace_of_plane(inner_plane) {
	vmovss 16(%r15), %xmm1
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/../../stdarch/crates/core_arch/src/x86/sse.rs:1130
		simd_bitmask::<i32x4, u8>(mask) as i32
	vmovmskps %xmm0, %eax
		// crates/impact_geometry/src/axis_aligned_box.rs:496
		!min_corner & 0b111
	notb %al
	vmovd %eax, %xmm2
	vgf2p8affineqb $0, .LCPI226_23(%rip){1to2}, %xmm2, %xmm2
	vmovd %xmm2, %eax
	shrb $5, %al
	movzbl %al, %eax
		// crates/impact_geometry/src/axis_aligned_box.rs:164
		let is_lower_y = (corner_idx >> 1) & 0b001 == 0;
	vpbroadcastq %rax, %xmm2
	vptestnmq .LCPI226_17(%rip), %xmm2, %k1
		// crates/impact_math/src/point.rs:312
		self.inner.y
	vmovsd 2084(%rsp), %xmm2
		// crates/impact_geometry/src/axis_aligned_box.rs:168
		if is_lower_x {
	andl $4, %eax
		// crates/impact_math/src/point.rs:312
		self.inner.y
	vmovaps %xmm2, %xmm3 {%k1}
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/glam-0.30.10/src/f32/sse2/vec3a.rs:109
		unsafe { UnionCast { a: [x, y, z, z] }.v }
	vmovss 2080(%rsp,%rax,4), %xmm2
	vpermt2ps %xmm2, %xmm4, %xmm3
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/../../stdarch/crates/core_arch/src/x86/sse.rs:88
		unsafe { simd_mul(a, b) }
	vmulps %xmm3, %xmm0, %xmm0
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/../../stdarch/crates/core_arch/src/x86/sse.rs:23
		unsafe { simd_insert!(a, 0, _mm_cvtss_f32(a) + _mm_cvtss_f32(b)) }
	vmovshdup %xmm0, %xmm2
	vaddss %xmm2, %xmm0, %xmm2
	vshufpd $1, %xmm0, %xmm0, %xmm0
	vaddss %xmm2, %xmm0, %xmm0
		// crates/impact_geometry/src/plane.rs:138
		self.compute_signed_distance(point) < 0.0
	vucomiss %xmm0, %xmm1
		// crates/impact_voxel/src/object/extraction.rs:767
		if !chunk_aabb.lies_in_negative_halfspace_of_plane(inner_plane) {
	ja .LBB226_125
		// crates/impact_voxel/src/object/extraction.rs:768
		intersecting_planes.push(normalized_face_planes[plane_idx]);
	cmpq 32(%rsp), %rbx
	jae .LBB226_436
	movq 40(%rsp), %rbp
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/allocator-api2-0.2.21/src/stable/vec/mod.rs:1793
		if self.len == self.buf.capacity() {
	cmpq %rbp, 480(%rsp)
	jne .LBB226_123
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/num/uint_macros.rs:896
		if intrinsics::unlikely(intrinsics::add_with_overflow(self, rhs).1) {
	cmpq $-1, %rbp
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/intrinsics/mod.rs:459
		if b {
	je .LBB226_418
	movq 40(%rsp), %rsi
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:1038
		if other < self { self } else { other }
	movl $4, %ebp
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/num/uint_macros.rs:896
		if intrinsics::unlikely(intrinsics::add_with_overflow(self, rhs).1) {
	leaq 1(%rsi), %rcx
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/allocator-api2-0.2.21/src/stable/raw_vec.rs:498
		let cap = cmp::max(self.cap * 2, required_cap);
	leaq (%rsi,%rsi), %rax
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:1038
		if other < self { self } else { other }
	cmpq %rax, %rcx
	cmovaq %rcx, %rax
	cmpq $5, %rax
	cmovaeq %rax, %rbp
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/alloc/layout.rs:573
		if element_size != 0 && n > Layout::max_size_for_alignment(alignment) / element_size {
	movq %rbp, %rdx
	shlq $4, %rdx
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/allocator-api2-0.2.21/src/stable/raw_vec.rs:333
		if mem::size_of::<T>() == 0 || self.cap == 0 {
	testq %rsi, %rsi
	je .LBB226_138
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/alloc/layout.rs:573
		if element_size != 0 && n > Layout::max_size_for_alignment(alignment) / element_size {
	cmpq 888(%rsp), %rax
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:966
		match self {
	ja .LBB226_418
		// crates/impact_alloc/src/arena.rs:169
		unsafe { &*self.arena_ptr }
	movq 64(%rsp), %rdi
	movq 40(%rsp), %rcx
	movq 16(%rdi), %rax
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/alloc/layout.rs:573
		if element_size != 0 && n > Layout::max_size_for_alignment(alignment) / element_size {
	shlq $4, %rcx
	movq %rcx, 240(%rsp)
	movq 32(%rax), %r13
	cmpq 176(%rsp), %r13
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:2355
		if align_is_compatible && self.is_last_allocation(ptr) {
	je .LBB226_145
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:1898
		let start = footer.data.as_ptr();
	movq (%rax), %rcx
.LBB226_135:
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mut_ptr.rs:470
		unsafe { intrinsics::arith_offset(self, count) as *mut T }
	andq $-4, %r13
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:1952
		if aligned_ptr < start || aligned_size > capacity {
	cmpq %rcx, %r13
	jb .LBB226_142
	movq %r13, %rsi
	subq %rcx, %rsi
	cmpq %rsi, %rdx
	ja .LBB226_142
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mut_ptr.rs:470
		unsafe { intrinsics::arith_offset(self, count) as *mut T }
	subq %rdx, %r13
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/mem/mod.rs:931
		crate::intrinsics::write_via_move(dest, src);
	movq %r13, 32(%rax)
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:1880
		if let Some(p) = self.try_alloc_layout_fast(layout) {
	jmp .LBB226_144
.LBB226_138:
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/alloc/layout.rs:573
		if element_size != 0 && n > Layout::max_size_for_alignment(alignment) / element_size {
	cmpq 888(%rsp), %rax
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:966
		match self {
	ja .LBB226_418
		// crates/impact_alloc/src/arena.rs:169
		unsafe { &*self.arena_ptr }
	movq 64(%rsp), %rdi
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cell.rs:555
		unsafe { *self.value.get() }
	movq 16(%rdi), %rax
	movq 32(%rax), %r13
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mut_ptr.rs:470
		unsafe { intrinsics::arith_offset(self, count) as *mut T }
	andq $-4, %r13
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:1952
		if aligned_ptr < start || aligned_size > capacity {
	movq %r13, %rcx
	subq (%rax), %rcx
	setb %sil
	cmpq $64, %rcx
	setb %cl
	orb %sil, %cl
	je .LBB226_149
	.cfi_escape 0x2e, 0x00
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:1883
		self.alloc_layout_slow(layout).ok_or(AllocErr)
	movl $4, %esi
	movq %rdx, 240(%rsp)
	vzeroupper
	callq *<bumpalo::Bump>::alloc_layout_slow@GOTPCREL(%rip)
	movq 240(%rsp), %rdx
	movq %rax, %r13
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:966
		match self {
	testq %rax, %rax
	jne .LBB226_124
	jmp .LBB226_438
.LBB226_142:
	.cfi_escape 0x2e, 0x00
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:1883
		self.alloc_layout_slow(layout).ok_or(AllocErr)
	movl $4, %esi
	movq %rdx, %r14
	vzeroupper
	callq *<bumpalo::Bump>::alloc_layout_slow@GOTPCREL(%rip)
	movq %rax, %r13
	movq %r14, %rdx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/option.rs:1335
		match self {
	testq %rax, %rax
	je .LBB226_438
.LBB226_144:
	.cfi_escape 0x2e, 0x00
	movq 176(%rsp), %rsi
	movq 240(%rsp), %rdx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mod.rs:551
		unsafe { crate::intrinsics::copy_nonoverlapping(src, dst, count) }
	movq %r13, %rdi
	vzeroupper
	callq *memcpy@GOTPCREL(%rip)
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:2371
		}
	jmp .LBB226_124
.LBB226_145:
	movabsq $9223372036854775793, %rsi
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:2358
		let delta = new_size - old_size;
	movq %rdx, %r8
	subq %rcx, %r8
	leaq 12(%rsi), %rcx
	cmpq %rcx, %r8
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/mem/alignment.rs:137
		if align.is_power_of_two() {
	jae .LBB226_438
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:1898
		let start = footer.data.as_ptr();
	movq (%rax), %rcx
	movq 176(%rsp), %rsi
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:453
		ptr.wrapping_sub(ptr as usize & (divisor - 1))
	andl $3, %esi
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mut_ptr.rs:470
		unsafe { intrinsics::arith_offset(self, count) as *mut T }
	movq %r13, %r14
	subq %rsi, %r14
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:1952
		if aligned_ptr < start || aligned_size > capacity {
	movq %r14, %rsi
	subq %rcx, %rsi
	jb .LBB226_135
	cmpq %rsi, %r8
	ja .LBB226_135
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mut_ptr.rs:470
		unsafe { intrinsics::arith_offset(self, count) as *mut T }
	subq %r8, %r14
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/mem/mod.rs:931
		crate::intrinsics::write_via_move(dest, src);
	movq %r14, 32(%rax)
	.cfi_escape 0x2e, 0x00
	movq 176(%rsp), %rsi
	movq 240(%rsp), %rdx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mod.rs:642
		crate::intrinsics::copy(src, dst, count)
	movq %r14, %rdi
	vzeroupper
	callq *memmove@GOTPCREL(%rip)
	movq %r14, %r13
	jmp .LBB226_124
.LBB226_149:
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mut_ptr.rs:470
		unsafe { intrinsics::arith_offset(self, count) as *mut T }
	addq $-64, %r13
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/mem/mod.rs:931
		crate::intrinsics::write_via_move(dest, src);
	movq %r13, 32(%rax)
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:1880
		if let Some(p) = self.try_alloc_layout_fast(layout) {
	jmp .LBB226_124
	.p2align	4
.LBB226_150:
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/allocator-api2-0.2.21/src/stable/vec/mod.rs:2043
		self.len() == 0
	cmpq $0, 480(%rsp)
	movq 88(%rsp), %rax
	movq 688(%rsp), %r12
		// crates/impact_voxel/src/object/extraction.rs:778
		if is_fully_inside {
	je .LBB226_238
	movq 256(%rsp), %r12
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/vec/mod.rs:3054
		let len = self.len;
	movq 368(%rsp), %rcx
	leaq 9(%r12), %r8
	movq %rcx, 624(%rsp)
		// crates/impact_voxel/src/object.rs:2535
		if let &mut Self::Uniform(UniformVoxelChunk {
	cmpb $4, (%r8)
	jne .LBB226_157
	movq %rax, %rdi
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/raw_vec/mod.rs:619
		if elem_size == 0 { usize::MAX } else { self.cap.as_inner() }
	movq 24(%rdi), %rcx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/vec/mod.rs:3054
		let len = self.len;
	movq 40(%rdi), %rbx
		// crates/impact_voxel/src/object.rs:2537
		split_detection,
	movl (%r12), %eax
		// crates/impact_voxel/src/object.rs:2536
		voxel,
	movzwl 4(%r12), %ebp
	movzbl 6(%r12), %r15d
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/num/uint_macros.rs:2621
		intrinsics::wrapping_sub(self, rhs)
	subq %rbx, %rcx
	movl %eax, 48(%rsp)
	movq %rbx, %rax
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/raw_vec/mod.rs:767
		additional > self.capacity(elem_layout.size()).wrapping_sub(len)
	cmpq $4095, %rcx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/raw_vec/mod.rs:673
		if self.needs_to_grow(len, additional, elem_layout) {
	jbe .LBB226_360
.LBB226_153:
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mut_ptr.rs:961
		unsafe { intrinsics::offset(self, count) }
	leaq (%rax,%rax,2), %rcx
	addq 32(%rdi), %rcx
	shll $16, %r15d
	xorl %edx, %edx
	orl %r15d, %ebp
	.p2align	4
.LBB226_154:
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mod.rs:1933
		intrinsics::write_via_move(dst, src)
	movl %ebp, %esi
	shrl $16, %esi
	movw %bp, (%rcx,%rdx)
	movb %sil, 2(%rcx,%rdx)
	movw %bp, 3(%rcx,%rdx)
	movb %sil, 5(%rcx,%rdx)
	movw %bp, 6(%rcx,%rdx)
	movb %sil, 8(%rcx,%rdx)
	movw %bp, 9(%rcx,%rdx)
	movb %sil, 11(%rcx,%rdx)
	movw %bp, 12(%rcx,%rdx)
	movb %sil, 14(%rcx,%rdx)
	movw %bp, 15(%rcx,%rdx)
	movb %sil, 17(%rcx,%rdx)
	movw %bp, 18(%rcx,%rdx)
	movb %sil, 20(%rcx,%rdx)
	movw %bp, 21(%rcx,%rdx)
	movb %sil, 23(%rcx,%rdx)
	movw %bp, 24(%rcx,%rdx)
	movb %sil, 26(%rcx,%rdx)
	movw %bp, 27(%rcx,%rdx)
	movb %sil, 29(%rcx,%rdx)
	movw %bp, 30(%rcx,%rdx)
	movb %sil, 32(%rcx,%rdx)
	movw %bp, 33(%rcx,%rdx)
	movb %sil, 35(%rcx,%rdx)
	movw %bp, 36(%rcx,%rdx)
	movb %sil, 38(%rcx,%rdx)
	movw %bp, 39(%rcx,%rdx)
	movb %sil, 41(%rcx,%rdx)
	movw %bp, 42(%rcx,%rdx)
	movb %sil, 44(%rcx,%rdx)
	movw %bp, 45(%rcx,%rdx)
	movb %sil, 47(%rcx,%rdx)
	movw %bp, 48(%rcx,%rdx)
	movb %sil, 50(%rcx,%rdx)
	movw %bp, 51(%rcx,%rdx)
	movb %sil, 53(%rcx,%rdx)
	movw %bp, 54(%rcx,%rdx)
	movb %sil, 56(%rcx,%rdx)
	movw %bp, 57(%rcx,%rdx)
	movb %sil, 59(%rcx,%rdx)
	movw %bp, 60(%rcx,%rdx)
	movb %sil, 62(%rcx,%rdx)
	movw %bp, 63(%rcx,%rdx)
	movb %sil, 65(%rcx,%rdx)
	movw %bp, 66(%rcx,%rdx)
	movb %sil, 68(%rcx,%rdx)
	movw %bp, 69(%rcx,%rdx)
	movb %sil, 71(%rcx,%rdx)
	movw %bp, 72(%rcx,%rdx)
	movb %sil, 74(%rcx,%rdx)
	movw %bp, 75(%rcx,%rdx)
	movb %sil, 77(%rcx,%rdx)
	movw %bp, 78(%rcx,%rdx)
	movb %sil, 80(%rcx,%rdx)
	movw %bp, 81(%rcx,%rdx)
	movb %sil, 83(%rcx,%rdx)
	movw %bp, 84(%rcx,%rdx)
	movb %sil, 86(%rcx,%rdx)
	movw %bp, 87(%rcx,%rdx)
	movb %sil, 89(%rcx,%rdx)
	movw %bp, 90(%rcx,%rdx)
	movb %sil, 92(%rcx,%rdx)
	movw %bp, 93(%rcx,%rdx)
	movb %sil, 95(%rcx,%rdx)
	movw %bp, 96(%rcx,%rdx)
	movb %sil, 98(%rcx,%rdx)
	movw %bp, 99(%rcx,%rdx)
	movb %sil, 101(%rcx,%rdx)
	movw %bp, 102(%rcx,%rdx)
	movb %sil, 104(%rcx,%rdx)
	movw %bp, 105(%rcx,%rdx)
	movb %sil, 107(%rcx,%rdx)
	movw %bp, 108(%rcx,%rdx)
	movb %sil, 110(%rcx,%rdx)
	movw %bp, 111(%rcx,%rdx)
	movb %sil, 113(%rcx,%rdx)
	movw %bp, 114(%rcx,%rdx)
	movb %sil, 116(%rcx,%rdx)
	movw %bp, 117(%rcx,%rdx)
	movb %sil, 119(%rcx,%rdx)
	movw %bp, 120(%rcx,%rdx)
	movb %sil, 122(%rcx,%rdx)
	movw %bp, 123(%rcx,%rdx)
	movb %sil, 125(%rcx,%rdx)
	movw %bp, 126(%rcx,%rdx)
	movb %sil, 128(%rcx,%rdx)
	movw %bp, 129(%rcx,%rdx)
	movb %sil, 131(%rcx,%rdx)
	movw %bp, 132(%rcx,%rdx)
	movb %sil, 134(%rcx,%rdx)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:1916
		fn lt(&self, other: &Self) -> bool { *self <  *other }
	addq $135, %rdx
	cmpq $12285, %rdx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/iter/range.rs:900
		if self.start < self.end {
	jne .LBB226_154
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/vec/set_len_on_drop.rs:19
		self.local_len += increment;
	addq $4096, %rax
		// crates/impact_voxel/src/object.rs:3154
		(start_voxel_idx >> (3 * LOG2_CHUNK_SIZE)) as u32
	shrq $12, %rbx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mod.rs:1933
		intrinsics::write_via_move(dst, src)
	movw %bp, (%rcx,%rdx)
	movb %sil, 2(%rcx,%rdx)
	movq %rax, 40(%rdi)
		// crates/impact_voxel/src/object.rs:2542
		*self = Self::NonUniform(NonUniformVoxelChunk {
	movl %ebx, (%r12)
	movl $65537, 4(%r12)
	movb $63, 8(%r12)
	movw $257, 4(%r8)
	movl $16843009, (%r8)
	.cfi_escape 0x2e, 0x00
	movq 16(%rsp), %rax
	movq 600(%rsp), %rdi
	movl 48(%rsp), %esi
	movq 8(%rsp), %r13
	movq %rax, (%rsp)
		// crates/impact_voxel/src/object.rs:2548
		split_detector.convert_uniform_chunk_to_non_uniform(split_detection);
	vzeroupper
	callq *<impact_voxel::object::split_detection::SplitDetector>::convert_uniform_chunk_to_non_uniform@GOTPCREL(%rip)
	movq 88(%rsp), %rax
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/vec/mod.rs:1903
		&mut *core::intrinsics::aggregate_raw_ptr::<*mut [T], _, _>(self.as_mut_ptr(), self.len)
	movq 16(%rax), %rcx
	movq %rcx, 48(%rsp)
.LBB226_157:
	movq 48(%rsp), %rsi
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/index.rs:278
		&mut (*slice)[self]
	cmpq %rsi, 288(%rsp)
	jae .LBB226_440
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/raw_vec/mod.rs:614
		self.ptr.cast().as_non_null_ptr()
	movq 8(%rax), %r15
	movq 472(%rsp), %rcx
		// crates/impact_voxel/src/object/extraction.rs:858
		let VoxelChunk::NonUniform(chunk) = &mut self.chunks[chunk_idx] else {
	cmpb $2, 9(%r15,%rcx)
	ja .LBB226_420
	addq %rcx, %r15
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/vec/mod.rs:1903
		&mut *core::intrinsics::aggregate_raw_ptr::<*mut [T], _, _>(self.as_mut_ptr(), self.len)
	movq 40(%rax), %rcx
		// crates/impact_voxel/src/object/extraction.rs:862
		let chunk_voxels = chunk_voxels_mut(&mut self.voxels, chunk.data_offset);
	movl (%r15), %r14d
	movq %rcx, 608(%rsp)
		// crates/impact_voxel/src/object.rs:3149
		(data_offset as usize) << (3 * LOG2_CHUNK_SIZE)
	movq %r14, %rdi
	shlq $12, %rdi
		// crates/impact_voxel/src/object.rs:3166
		&mut voxels[start_voxel_idx..start_voxel_idx + CHUNK_VOXEL_COUNT]
	leaq 4096(%rdi), %rbx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/index.rs:451
		&& self.end <= slice.len()
	cmpq %rcx, %rbx
	ja .LBB226_421
		// crates/impact_voxel/src/object/extraction.rs:862
		let chunk_voxels = chunk_voxels_mut(&mut self.voxels, chunk.data_offset);
	movq 32(%rax), %r13
	movq 624(%rsp), %r12
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/raw_vec/mod.rs:619
		if elem_size == 0 { usize::MAX } else { self.cap.as_inner() }
	movq 352(%rsp), %rax
	movq %rdi, %rbp
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/num/uint_macros.rs:2621
		intrinsics::wrapping_sub(self, rhs)
	subq %r12, %rax
	movq %r12, %rbx
	movq %r13, 616(%rsp)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/raw_vec/mod.rs:767
		additional > self.capacity(elem_layout.size()).wrapping_sub(len)
	cmpq $4095, %rax
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/raw_vec/mod.rs:673
		if self.needs_to_grow(len, additional, elem_layout) {
	jbe .LBB226_356
.LBB226_161:
	movq %r15, 880(%rsp)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/raw_vec/mod.rs:614
		self.ptr.cast().as_non_null_ptr()
	movq 360(%rsp), %r15
	leaq (%rbp,%rbp,2), %rbp
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mut_ptr.rs:961
		unsafe { intrinsics::offset(self, count) }
	leaq (%rbx,%rbx,2), %rax
	addq %r13, %rbp
	movq %rax, 1232(%rsp)
	leaq (%r15,%rax), %rdi
	.cfi_escape 0x2e, 0x00
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mod.rs:551
		unsafe { crate::intrinsics::copy_nonoverlapping(src, dst, count) }
	movl $12288, %edx
	movq %rbp, %rsi
	vzeroupper
	callq *memcpy@GOTPCREL(%rip)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/vec/mod.rs:2939
		self.len += count;
	addq $4096, %rbx
	movq %rbx, 368(%rsp)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/index.rs:580
		if self.start > slice.len() {
	cmpq %rbx, %r12
	ja .LBB226_422
		// crates/impact_math/src/point.rs:306
		self.inner.x
	vmovsd 2080(%rsp), %xmm1
		// crates/impact_math/src/point.rs:660
		Point3C::new(a.x + b.x(), a.y + b.y(), a.z + b.z())
	vmovss .LCPI226_12(%rip), %xmm0
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/index.rs:101
		let ptr = unsafe { crate::intrinsics::offset(ptr, offset) };
	leaq (%r12,%r12,2), %rcx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/array/mod.rs:598
		self.try_map(NeverShortCircuit::wrap_mut_1(f)).0
	movq 208(%rsp), %rdx
	movq 216(%rsp), %rsi
	movq 224(%rsp), %rdi
	movl $16, 472(%rsp)
	movq $0, 664(%rsp)
	movq %rbp, 1224(%rsp)
	movq $0, 680(%rsp)
	movq $0, 640(%rsp)
	movq $0, 672(%rsp)
	movq $0, 648(%rsp)
	movq $0, 656(%rsp)
	movq $0, 912(%rsp)
	movq $0, 952(%rsp)
	movq $0, 936(%rsp)
	movq $0, 944(%rsp)
	movq $0, 920(%rsp)
	movq $0, 928(%rsp)
	movq $0, 968(%rsp)
	movq $0, 976(%rsp)
	movl $16, 488(%rsp)
	movl $16, 340(%rsp)
	movq $0, 960(%rsp)
	movl $16, 348(%rsp)
	movl $16, 344(%rsp)
	movl $16, 336(%rsp)
	movq $0, 448(%rsp)
	movq $0, 456(%rsp)
	movq $0, 464(%rsp)
	movl $16, 532(%rsp)
	movl $0, 528(%rsp)
	movl $16, 524(%rsp)
	movl $0, 520(%rsp)
	movq $0, 1048(%rsp)
	movq $0, 1040(%rsp)
	movl $16, 516(%rsp)
	movl $0, 512(%rsp)
	movl $0, 508(%rsp)
	movl $16, 504(%rsp)
	movl $0, 500(%rsp)
	movl $16, 496(%rsp)
	movl $0, 492(%rsp)
	movq $0, 1032(%rsp)
	movq $0, 1024(%rsp)
	movq $0, 1016(%rsp)
	movq $0, 1008(%rsp)
	movq $0, 1000(%rsp)
	movq $0, 992(%rsp)
	movq $0, 1064(%rsp)
	movq $0, 984(%rsp)
	movq $0, 1056(%rsp)
		// crates/impact_math/src/point.rs:660
		Point3C::new(a.x + b.x(), a.y + b.y(), a.z + b.z())
	vaddps .LCPI226_12(%rip){1to4}, %xmm1, %xmm1
	vaddss 2088(%rsp), %xmm0, %xmm0
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/index.rs:101
		let ptr = unsafe { crate::intrinsics::offset(ptr, offset) };
	leaq (%r15,%rcx), %rax
	movq %rcx, 1208(%rsp)
	movq %rax, 1216(%rsp)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/iter/range.rs:900
		if self.start < self.end {
	leaq 2(%r15,%rcx), %rax
	movq %rax, 688(%rsp)
	leaq (%r14,%r14,2), %rax
		// crates/impact_voxel/src/object/extraction.rs:867
		let chunk_start_voxel_indices = chunk_indices.map(|idx| idx * CHUNK_SIZE);
	shlq $4, %rdx
	shlq $4, %rsi
	shlq $4, %rdi
	movq %rdx, 1256(%rsp)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/iter/range.rs:900
		if self.start < self.end {
	leaq -1(%r15,%rcx), %rdx
	movq %rsi, 544(%rsp)
	movq %rdi, 200(%rsp)
	movq %rbp, %r15
	shlq $12, %rax
	leaq 2(%r13,%rax), %rax
	movq %rdx, %r14
	xorl %r13d, %r13d
	movq %rax, 1288(%rsp)
	xorl %eax, %eax
		// crates/impact_math/src/point.rs:499
		Self { x, y, z }
	vmovlps %xmm1, 1072(%rsp)
		// crates/impact_voxel/src/object/extraction.rs:876
		[VoxelSignedDistance::maximally_inside(); CHUNK_SIZE];
	vmovddup .LCPI226_25(%rip), %xmm1
		// crates/impact_math/src/point.rs:499
		Self { x, y, z }
	vmovss %xmm0, 1080(%rsp)
		// crates/impact_voxel/src/object/extraction.rs:876
		[VoxelSignedDistance::maximally_inside(); CHUNK_SIZE];
	vmovaps %xmm1, 1760(%rsp)
	jmp .LBB226_164
	.p2align	4
.LBB226_163:
	movq 1280(%rsp), %r14
	movq 1272(%rsp), %r15
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/iter/range.rs:900
		if self.start < self.end {
	addq %rcx, %r12
	movq 1264(%rsp), %rcx
	movq %r12, %rax
	addq $768, %r14
	addq $768, %r15
	movq %rcx, %r13
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:1916
		fn lt(&self, other: &Self) -> bool { *self <  *other }
	cmpq $16, %rcx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/iter/range.rs:900
		if self.start < self.end {
	je .LBB226_214
.LBB226_164:
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/num/uint_macros.rs:971
		intrinsics::unchecked_add(self, rhs)
	leaq 1(%r13), %rcx
	xorl %ebx, %ebx
	movq %r15, 1272(%rsp)
	movq %r14, 1280(%rsp)
	movq %rcx, 1264(%rsp)
	movq 1256(%rsp), %rcx
		// crates/impact_voxel/src/object/extraction.rs:896
		let obj_i = chunk_start_voxel_indices[0] + i as usize;
	addq %r13, %rcx
	vcvtusi2ss %rcx, %xmm15, %xmm0
	vmulss 560(%rsp), %xmm0, %xmm0
	vmovaps %xmm0, 784(%rsp)
	jmp .LBB226_167
	.p2align	4
.LBB226_165:
	movq 1056(%rsp), %rdx
		// crates/impact_voxel/src/object.rs:2925
		self.0[2][0] += 1;
	incq %rdx
	movq %rdx, 1056(%rsp)
	movq %rdx, 952(%rsp)
.LBB226_166:
	leaq (%r12,%rcx), %rax
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/iter/range.rs:900
		if self.start < self.end {
	addq $48, %r14
	addq $48, %r15
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:1916
		fn lt(&self, other: &Self) -> bool { *self <  *other }
	cmpq $16, %rbx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/iter/range.rs:900
		if self.start < self.end {
	je .LBB226_163
.LBB226_167:
	movq %r15, 80(%rsp)
	movq %rax, %r15
	.cfi_escape 0x2e, 0x00
	movq 176(%rsp), %rsi
	movq 480(%rsp), %rdx
		// crates/impact_voxel/src/object/extraction.rs:907
		Self::compute_max_plane_signed_dists_for_row(
	leaq 1760(%rsp), %rdi
	leaq 1072(%rsp), %rcx
	movl %r13d, %r8d
	movl %ebx, %r9d
	callq <impact_voxel::object::VoxelObject>::compute_max_plane_signed_dists_for_row
	leaq 1(%rbx), %rax
	vmovaps 784(%rsp), %xmm1
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/iter/range.rs:900
		if self.start < self.end {
	leaq (%r15,%r15,2), %r11
	movl $16, %edx
	movq %r15, 240(%rsp)
	movq $0, 184(%rsp)
	xorl %r12d, %r12d
	xorl %ecx, %ecx
	xorl %esi, %esi
	movq $0, 256(%rsp)
	movl $0, (%rsp)
	movl $16, 48(%rsp)
	movq %rax, 192(%rsp)
	movq 544(%rsp), %rax
		// crates/impact_voxel/src/object/extraction.rs:902
		let obj_j = chunk_start_voxel_indices[1] + j as usize;
	addq %rbx, %rax
	vcvtusi2ss %rax, %xmm15, %xmm0
	vmulss 560(%rsp), %xmm0, %xmm0
	movq 688(%rsp), %rax
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/iter/range.rs:900
		if self.start < self.end {
	addq %r11, %rax
	addq 1288(%rsp), %r11
	vinsertps $16, %xmm0, %xmm1, %xmm0
	jmp .LBB226_170
	.p2align	4
.LBB226_169:
		// crates/impact_voxel/src/object/extraction.rs:1013
		*NonUniformVoxelChunk::get_voxel_mut(
	movb %bpl, -2(%rax,%r12)
	movb %r10b, -1(%rax,%r12)
	movb %dil, (%rax,%r12)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:1916
		fn lt(&self, other: &Self) -> bool { *self <  *other }
	addq $3, %r12
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/num/uint_macros.rs:971
		intrinsics::unchecked_add(self, rhs)
	incq %rcx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:1916
		fn lt(&self, other: &Self) -> bool { *self <  *other }
	cmpq $48, %r12
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/iter/range.rs:900
		if self.start < self.end {
	je .LBB226_195
.LBB226_170:
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:1038
		if other < self { self } else { other }
	movzbl 1760(%rsp,%rcx), %edi
	movzbl -1(%r11,%r12), %r10d
	movq %r13, %r15
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/num/int_macros.rs:1960
		intrinsics::saturating_add(self, rhs)
	movl $127, %r13d
		// crates/impact_voxel/src/object/extraction.rs:930
		let mut poly_voxel = *voxel;
	movzbl (%r11,%r12), %r8d
	movzbl -2(%r11,%r12), %ebp
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/num/int_macros.rs:1960
		intrinsics::saturating_add(self, rhs)
	movl %edi, %r9d
	incb %r9b
	movzbl %r9b, %r9d
	cmovol %r13d, %r9d
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/num/int_macros.rs:2046
		intrinsics::saturating_sub(0, self)
	negb %r9b
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:1038
		if other < self { self } else { other }
	movzbl %r9b, %r9d
	cmpb %r10b, %r9b
	cmovlel %r10d, %r9d
	cmpb %r10b, %dil
	cmovgl %edi, %r10d
		// crates/impact_voxel/src/lib.rs:75
		bitflags! {
	movl %r8d, %edi
		// crates/impact_voxel/src/object/extraction.rs:950
		voxel.signed_distance = voxel_signed_distance;
	movb %r9b, -1(%r11,%r12)
		// crates/impact_voxel/src/lib.rs:75
		bitflags! {
	orb $1, %dil
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/num/int_macros.rs:3750
		pub const fn is_negative(self) -> bool { self < 0 }
	testb %r10b, %r10b
		// crates/impact_voxel/src/object/extraction.rs:954
		if poly_voxel_signed_distance.is_negative() {
	js .LBB226_179
	movq %r15, %r13
	testq %r15, %r15
		// crates/impact_voxel/src/object/extraction.rs:1875
		if i > 0 {
	je .LBB226_173
		// crates/impact_voxel/src/lib.rs:75
		bitflags! {
	andb $-33, -765(%r14,%r12)
.LBB226_173:
	testq %rbx, %rbx
		// crates/impact_voxel/src/object/extraction.rs:1878
		if j > 0 {
	je .LBB226_175
		// crates/impact_voxel/src/lib.rs:75
		bitflags! {
	andb $-65, -45(%r14,%r12)
.LBB226_175:
		// crates/impact_voxel/src/object/extraction.rs:1881
		if k > 0 {
	testq %r12, %r12
	je .LBB226_177
		// crates/impact_voxel/src/lib.rs:75
		bitflags! {
	andb $127, (%r14,%r12)
.LBB226_177:
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/num/int_macros.rs:3750
		pub const fn is_negative(self) -> bool { self < 0 }
	testb %r9b, %r9b
		// crates/impact_voxel/src/object/extraction.rs:999
		if voxel_signed_distance.is_negative() {
	jns .LBB226_169
	movl 48(%rsp), %r8d
	movl (%rsp), %r9d
		// crates/impact_voxel/src/object/extraction.rs:1000
		row_occupied_count += 1;
	incq 256(%rsp)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:1077
		if other < self { other } else { self }
	cmpl %r8d, %ecx
	cmovbl %ecx, %r8d
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:1038
		if other < self { self } else { other }
	cmpl %r9d, %ecx
	cmoval %ecx, %r9d
	movl %r8d, 48(%rsp)
	movl %r9d, (%rsp)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:1039
		}
	jmp .LBB226_169
	.p2align	4
.LBB226_179:
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:1077
		if other < self { other } else { self }
	cmpl %edx, %ecx
	movzbl %bpl, %r9d
		// crates/impact_voxel/src/lib.rs:75
		bitflags! {
	movb %dil, (%r11,%r12)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:1077
		if other < self { other } else { self }
	cmovbl %ecx, %edx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:1038
		if other < self { self } else { other }
	cmpl %esi, %ecx
	cmoval %ecx, %esi
		// crates/impact_voxel/src/object/inertia.rs:586
		let voxel_density = voxel_type_densities[voxel_type.idx()];
	cmpq %r9, 248(%rsp)
	jbe .LBB226_416
	movq 200(%rsp), %rdi
	vmovaps 560(%rsp), %xmm3
	movq 264(%rsp), %r13
	addq %rcx, %rdi
	vmovss (%r13,%r9,4), %xmm1
	movq 152(%rsp), %r9
	movq %r15, %r13
		// crates/impact_voxel/src/object/inertia.rs:588
		let lower_coords = Vector3::from(object_voxel_indices.map(|index| voxel_extent * index as f32));
	vcvtusi2ss %rdi, %xmm15, %xmm2
		// crates/impact_voxel/src/object/inertia.rs:604
		let moments_of_inertia = ((1.0 / 3.0) * voxel_extent_pow_2 * voxel_density)
	vmulss 716(%rsp), %xmm1, %xmm6
	movq 320(%rsp), %rdi
		// crates/impact_voxel/src/object/inertia.rs:588
		let lower_coords = Vector3::from(object_voxel_indices.map(|index| voxel_extent * index as f32));
	vmulss %xmm2, %xmm3, %xmm2
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/glam-0.30.10/src/f32/sse2/vec3a.rs:109
		unsafe { UnionCast { a: [x, y, z, z] }.v }
	vshufps $4, %xmm2, %xmm0, %xmm2
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/../../stdarch/crates/core_arch/src/x86/sse.rs:36
		unsafe { simd_add(a, b) }
	vaddps %xmm2, %xmm3, %xmm3
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/../../stdarch/crates/core_arch/src/x86/sse.rs:88
		unsafe { simd_mul(a, b) }
	vmulps %xmm2, %xmm2, %xmm4
	vmulps %xmm3, %xmm3, %xmm5
	vmulps %xmm4, %xmm2, %xmm2
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/../../stdarch/crates/core_arch/src/simd.rs:57
		unsafe { crate::intrinsics::simd::simd_splat(value) }
	vbroadcastss %xmm6, %xmm6
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/../../stdarch/crates/core_arch/src/x86/sse.rs:88
		unsafe { simd_mul(a, b) }
	vmulps %xmm5, %xmm3, %xmm3
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/../../stdarch/crates/core_arch/src/x86/sse.rs:62
		unsafe { simd_sub(a, b) }
	vsubps %xmm2, %xmm3, %xmm2
	vsubps %xmm4, %xmm5, %xmm3
		// crates/impact_voxel/src/object/inertia.rs:602
		let moments = (0.5 * voxel_extent_pow_2 * voxel_density) * squared_coord_diff;
	vmulss 552(%rsp), %xmm1, %xmm5
		// crates/impact_voxel/src/object/inertia.rs:600
		let mass = voxel_extent_pow_3 * voxel_density;
	vmulss 312(%rsp), %xmm1, %xmm4
		// crates/impact_voxel/src/object/inertia.rs:607
		let products_of_inertia = (0.25 * voxel_extent * voxel_density)
	vmulss 712(%rsp), %xmm1, %xmm1
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/../../stdarch/crates/core_arch/src/macros.rs:173
		$crate::intrinsics::simd::simd_shuffle(
	vshufps $1, %xmm2, %xmm2, %xmm7
	vshufps $26, %xmm2, %xmm2, %xmm2
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/../../stdarch/crates/core_arch/src/x86/sse.rs:36
		unsafe { simd_add(a, b) }
	vaddps %xmm2, %xmm7, %xmm2
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/../../stdarch/crates/core_arch/src/x86/sse.rs:88
		unsafe { simd_mul(a, b) }
	vmulps %xmm6, %xmm2, %xmm2
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/../../stdarch/crates/core_arch/src/macros.rs:173
		$crate::intrinsics::simd::simd_shuffle(
	vshufps $9, %xmm3, %xmm3, %xmm6
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/../../stdarch/crates/core_arch/src/simd.rs:57
		unsafe { crate::intrinsics::simd::simd_splat(value) }
	vbroadcastss %xmm5, %xmm5
	vbroadcastss %xmm1, %xmm1
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/../../stdarch/crates/core_arch/src/x86/sse.rs:88
		unsafe { simd_mul(a, b) }
	vmulps %xmm5, %xmm3, %xmm5
	vmulps %xmm6, %xmm3, %xmm3
	vmulps %xmm1, %xmm3, %xmm1
		// crates/impact_voxel/src/object/inertia.rs:413
		self.source.mass -= voxel_mass;
	vmovss 48(%rdi), %xmm3
	vsubss %xmm4, %xmm3, %xmm3
	vmovss %xmm3, 48(%rdi)
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/glam-0.30.10/src/f32/sse2/vec3a.rs:1733
		self.0 = unsafe { _mm_sub_ps(self.0, rhs.0) };
	vmovaps (%rdi), %xmm3
	vmovaps 16(%rdi), %xmm6
	vmovaps 32(%rdi), %xmm7
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/../../stdarch/crates/core_arch/src/x86/sse.rs:62
		unsafe { simd_sub(a, b) }
	vsubps %xmm5, %xmm3, %xmm3
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/glam-0.30.10/src/f32/sse2/vec3a.rs:1733
		self.0 = unsafe { _mm_sub_ps(self.0, rhs.0) };
	vmovaps %xmm3, (%rdi)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/../../stdarch/crates/core_arch/src/x86/sse.rs:62
		unsafe { simd_sub(a, b) }
	vsubps %xmm2, %xmm6, %xmm3
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/glam-0.30.10/src/f32/sse2/vec3a.rs:1733
		self.0 = unsafe { _mm_sub_ps(self.0, rhs.0) };
	vmovaps %xmm3, 16(%rdi)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/../../stdarch/crates/core_arch/src/x86/sse.rs:62
		unsafe { simd_sub(a, b) }
	vsubps %xmm1, %xmm7, %xmm3
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/glam-0.30.10/src/f32/sse2/vec3a.rs:1733
		self.0 = unsafe { _mm_sub_ps(self.0, rhs.0) };
	vmovaps %xmm3, 32(%rdi)
	movq 80(%rsp), %rdi
		// crates/impact_voxel/src/object/inertia.rs:418
		self.destination.mass += voxel_mass;
	vaddss 48(%r9), %xmm4, %xmm3
	vmovss %xmm3, 48(%r9)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/../../stdarch/crates/core_arch/src/x86/sse.rs:36
		unsafe { simd_add(a, b) }
	vaddps (%r9), %xmm5, %xmm3
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/glam-0.30.10/src/f32/sse2/vec3a.rs:1609
		self.0 = unsafe { _mm_add_ps(self.0, rhs.0) };
	vmovaps %xmm3, (%r9)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/../../stdarch/crates/core_arch/src/x86/sse.rs:36
		unsafe { simd_add(a, b) }
	vaddps 16(%r9), %xmm2, %xmm2
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/glam-0.30.10/src/f32/sse2/vec3a.rs:1609
		self.0 = unsafe { _mm_add_ps(self.0, rhs.0) };
	vmovaps %xmm2, 16(%r9)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/../../stdarch/crates/core_arch/src/x86/sse.rs:36
		unsafe { simd_add(a, b) }
	vaddps 32(%r9), %xmm1, %xmm1
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/glam-0.30.10/src/f32/sse2/vec3a.rs:1609
		self.0 = unsafe { _mm_add_ps(self.0, rhs.0) };
	vmovaps %xmm1, 32(%r9)
	testq %r15, %r15
		// crates/impact_voxel/src/object/extraction.rs:1895
		if i > 0 {
	je .LBB226_182
		// crates/impact_voxel/src/lib.rs:75
		bitflags! {
	andb $-33, -766(%rdi,%r12)
	cmpq $15, %r13
		// crates/impact_voxel/src/object/extraction.rs:1898
		if i + 1 < CHUNK_SIZE {
	je .LBB226_183
.LBB226_182:
		// crates/impact_voxel/src/lib.rs:75
		bitflags! {
	andb $-5, 770(%rdi,%r12)
.LBB226_183:
	testq %rbx, %rbx
		// crates/impact_voxel/src/object/extraction.rs:1901
		if j > 0 {
	je .LBB226_185
		// crates/impact_voxel/src/lib.rs:75
		bitflags! {
	andb $-65, -46(%rdi,%r12)
	cmpq $15, %rbx
		// crates/impact_voxel/src/object/extraction.rs:1904
		if j + 1 < CHUNK_SIZE {
	je .LBB226_186
.LBB226_185:
		// crates/impact_voxel/src/lib.rs:75
		bitflags! {
	andb $-9, 50(%rdi,%r12)
.LBB226_186:
		// crates/impact_voxel/src/object/extraction.rs:1907
		if k > 0 {
	testq %r12, %r12
	je .LBB226_188
		// crates/impact_voxel/src/lib.rs:75
		bitflags! {
	andb $127, -1(%rdi,%r12)
		// crates/impact_voxel/src/object/extraction.rs:1910
		if k + 1 < CHUNK_SIZE {
	cmpq $45, %r12
	je .LBB226_189
.LBB226_188:
		// crates/impact_voxel/src/lib.rs:75
		bitflags! {
	andb $-17, 5(%rdi,%r12)
.LBB226_189:
	testq %r13, %r13
		// crates/impact_voxel/src/object/extraction.rs:1843
		if i > 0 {
	je .LBB226_194
		// crates/impact_voxel/src/object/extraction.rs:1832
		let is_negative_mask = adjacent_voxel.signed_distance.is_negative_mask();
	movzbl -766(%r14,%r12), %r9d
		// crates/impact_voxel/src/lib.rs:75
		bitflags! {
	andb $-6, %r8b
		// crates/impact_voxel/src/lib.rs:234
		(self.encoded >> 7) as u8
	sarb $7, %r9b
		// crates/impact_voxel/src/lib.rs:346
		Self::from_bits_retain(self.bits() & mask)
	movl %r9d, %edi
	andb $32, %r9b
		// crates/impact_voxel/src/lib.rs:75
		bitflags! {
	orb %r9b, -765(%r14,%r12)
		// crates/impact_voxel/src/lib.rs:346
		Self::from_bits_retain(self.bits() & mask)
	andb $4, %dil
		// crates/impact_voxel/src/lib.rs:75
		bitflags! {
	orb %r8b, %dil
	testq %rbx, %rbx
		// crates/impact_voxel/src/object/extraction.rs:1850
		if j > 0 {
	je .LBB226_192
.LBB226_191:
		// crates/impact_voxel/src/object/extraction.rs:1832
		let is_negative_mask = adjacent_voxel.signed_distance.is_negative_mask();
	movzbl -46(%r14,%r12), %r8d
		// crates/impact_voxel/src/lib.rs:75
		bitflags! {
	andb $-9, %dil
		// crates/impact_voxel/src/lib.rs:234
		(self.encoded >> 7) as u8
	sarb $7, %r8b
		// crates/impact_voxel/src/lib.rs:346
		Self::from_bits_retain(self.bits() & mask)
	movl %r8d, %r9d
	andb $64, %r8b
		// crates/impact_voxel/src/lib.rs:75
		bitflags! {
	orb %r8b, -45(%r14,%r12)
		// crates/impact_voxel/src/lib.rs:346
		Self::from_bits_retain(self.bits() & mask)
	andb $8, %r9b
		// crates/impact_voxel/src/lib.rs:75
		bitflags! {
	orb %r9b, %dil
.LBB226_192:
	incq 184(%rsp)
		// crates/impact_voxel/src/object/extraction.rs:1907
		if k > 0 {
	testq %r12, %r12
		// crates/impact_voxel/src/object/extraction.rs:1857
		if k > 0 {
	je .LBB226_169
		// crates/impact_voxel/src/object/extraction.rs:1832
		let is_negative_mask = adjacent_voxel.signed_distance.is_negative_mask();
	movzbl -1(%r14,%r12), %r8d
		// crates/impact_voxel/src/lib.rs:75
		bitflags! {
	andb $-17, %dil
		// crates/impact_voxel/src/lib.rs:234
		(self.encoded >> 7) as u8
	sarb $7, %r8b
		// crates/impact_voxel/src/lib.rs:346
		Self::from_bits_retain(self.bits() & mask)
	movl %r8d, %r9d
	andb $-128, %r8b
		// crates/impact_voxel/src/lib.rs:75
		bitflags! {
	orb %r8b, (%r14,%r12)
		// crates/impact_voxel/src/lib.rs:346
		Self::from_bits_retain(self.bits() & mask)
	andb $16, %r9b
		// crates/impact_voxel/src/lib.rs:75
		bitflags! {
	orb %r9b, %dil
	jmp .LBB226_169
.LBB226_194:
	andb $-2, %r8b
	movl %r8d, %edi
	testq %rbx, %rbx
		// crates/impact_voxel/src/object/extraction.rs:1850
		if j > 0 {
	jne .LBB226_191
	jmp .LBB226_192
	.p2align	4
.LBB226_195:
	movl (%rsp), %ebp
	movl 48(%rsp), %r11d
		// crates/impact_voxel/src/object/extraction.rs:1021
		if lower_occupied_k <= upper_occupied_k {
	cmpl %ebp, %r11d
	ja .LBB226_197
	movl 532(%rsp), %edi
	movl 528(%rsp), %r8d
	movl 516(%rsp), %r9d
	movl 512(%rsp), %r10d
	movl 472(%rsp), %eax
	movl 508(%rsp), %r15d
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:1077
		if other < self { other } else { self }
	cmpl %edi, %r13d
	cmovbl %r13d, %edi
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:1038
		if other < self { self } else { other }
	cmpl %r8d, %r13d
	cmoval %r13d, %r8d
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:1077
		if other < self { other } else { self }
	cmpl %r9d, %ebx
	movl %edi, 532(%rsp)
	movl %edi, 340(%rsp)
	cmovbl %ebx, %r9d
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:1038
		if other < self { self } else { other }
	cmpl %r10d, %ebx
	movl %r8d, 528(%rsp)
	cmoval %ebx, %r10d
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:1077
		if other < self { other } else { self }
	cmpl %eax, %r11d
	movl %r9d, 516(%rsp)
	movl %r9d, 488(%rsp)
	cmovbl %r11d, %eax
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:1038
		if other < self { self } else { other }
	cmpl %r15d, %ebp
	movl %r10d, 512(%rsp)
	cmoval %ebp, %r15d
	movl %eax, 472(%rsp)
	movl %r10d, %eax
	movq %rax, 968(%rsp)
	movl %r15d, %eax
	movq %rax, 976(%rsp)
	movl %r8d, %eax
	movl %r15d, 508(%rsp)
	movq %rax, 960(%rsp)
.LBB226_197:
	movq 240(%rsp), %r12
		// crates/impact_voxel/src/object/extraction.rs:1033
		if poly_lower_occupied_k <= poly_upper_occupied_k {
	cmpl %esi, %edx
	ja .LBB226_199
	movl 524(%rsp), %eax
	movl 520(%rsp), %edi
	movl 504(%rsp), %r8d
	movl 500(%rsp), %r9d
	movl 496(%rsp), %r10d
	movl 492(%rsp), %r11d
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:1077
		if other < self { other } else { self }
	cmpl %eax, %r13d
	cmovbl %r13d, %eax
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:1038
		if other < self { self } else { other }
	cmpl %edi, %r13d
	cmoval %r13d, %edi
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:1077
		if other < self { other } else { self }
	cmpl %r8d, %ebx
	movl %eax, 524(%rsp)
	movl %eax, 348(%rsp)
	cmovbl %ebx, %r8d
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:1038
		if other < self { self } else { other }
	cmpl %r9d, %ebx
	movl %edi, 520(%rsp)
	cmoval %ebx, %r9d
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:1077
		if other < self { other } else { self }
	cmpl %r10d, %edx
	movl %r8d, 504(%rsp)
	movl %r8d, 344(%rsp)
	cmovbl %edx, %r10d
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:1038
		if other < self { self } else { other }
	cmpl %r11d, %esi
	movl %r9d, 500(%rsp)
	cmoval %esi, %r11d
	movl %r10d, 496(%rsp)
	movl %r10d, 336(%rsp)
	movl %r11d, %eax
	movq %rax, 448(%rsp)
	movl %r9d, %eax
	movq %rax, 456(%rsp)
	movl %edi, %eax
	movl %r11d, 492(%rsp)
	movq %rax, 464(%rsp)
.LBB226_199:
	movl 48(%rsp), %r9d
	movq 184(%rsp), %r10
	movq 256(%rsp), %r11
		// crates/impact_voxel/src/object/extraction.rs:1049
		if on_lower_x_face {
	testl %r13d, %r13d
	je .LBB226_209
	cmpl $15, %r13d
	jne .LBB226_202
	movq 1032(%rsp), %rdi
	movq 1024(%rsp), %r8
		// crates/impact_voxel/src/object.rs:2938
		self.0[0][1] += count;
	addq %r11, %rdi
	addq %r10, %r8
	movq %rdi, 1032(%rsp)
	movq %rdi, 648(%rsp)
	movq %r8, 1024(%rsp)
	movq %r8, 920(%rsp)
.LBB226_202:
		// crates/impact_voxel/src/object/extraction.rs:1056
		if on_lower_y_face {
	testl %ebx, %ebx
	je .LBB226_210
.LBB226_203:
	cmpl $15, %ebx
	jne .LBB226_205
	movq 1016(%rsp), %rdi
		// crates/impact_voxel/src/object.rs:2946
		self.0[1][1] += count;
	addq %r11, 664(%rsp)
	addq %r10, %rdi
	movq %rdi, 1016(%rsp)
	movq %rdi, 936(%rsp)
.LBB226_205:
	movq 192(%rsp), %rbx
		// crates/impact_voxel/src/object/extraction.rs:1063
		if lower_occupied_k == 0 {
	testl %r9d, %r9d
	je .LBB226_211
.LBB226_206:
		// crates/impact_voxel/src/object/extraction.rs:1065
		} else if upper_occupied_k == CHUNK_SIZE_U32 - 1 {
	cmpl $15, %ebp
	jne .LBB226_208
	movq 992(%rsp), %rdi
		// crates/impact_voxel/src/object.rs:2929
		self.0[2][1] += 1;
	incq %rdi
	movq %rdi, 992(%rsp)
	movq %rdi, 640(%rsp)
.LBB226_208:
	movq 80(%rsp), %r15
		// crates/impact_voxel/src/object/extraction.rs:1068
		if poly_lower_occupied_k == 0 {
	testl %edx, %edx
	jne .LBB226_212
	jmp .LBB226_165
	.p2align	4
.LBB226_209:
	movq 1048(%rsp), %rdi
	movq 1040(%rsp), %r8
		// crates/impact_voxel/src/object.rs:2934
		self.0[0][0] += count;
	addq %r11, %rdi
	addq %r10, %r8
	movq %rdi, 1048(%rsp)
	movq %rdi, 656(%rsp)
	movq %r8, 1040(%rsp)
	movq %r8, 928(%rsp)
		// crates/impact_voxel/src/object/extraction.rs:1056
		if on_lower_y_face {
	testl %ebx, %ebx
	jne .LBB226_203
.LBB226_210:
	movq 1008(%rsp), %rdi
	movq 1000(%rsp), %r8
		// crates/impact_voxel/src/object.rs:2942
		self.0[1][0] += count;
	addq %r11, %rdi
	addq %r10, %r8
	movq %rdi, 1008(%rsp)
	movq %rdi, 672(%rsp)
	movq %r8, 1000(%rsp)
	movq %r8, 944(%rsp)
	movq 192(%rsp), %rbx
		// crates/impact_voxel/src/object/extraction.rs:1063
		if lower_occupied_k == 0 {
	testl %r9d, %r9d
	jne .LBB226_206
.LBB226_211:
	movq 1064(%rsp), %rdi
		// crates/impact_voxel/src/object.rs:2925
		self.0[2][0] += 1;
	incq %rdi
	movq %rdi, 1064(%rsp)
	movq %rdi, 680(%rsp)
	movq 80(%rsp), %r15
		// crates/impact_voxel/src/object/extraction.rs:1068
		if poly_lower_occupied_k == 0 {
	testl %edx, %edx
	je .LBB226_165
.LBB226_212:
		// crates/impact_voxel/src/object/extraction.rs:1070
		} else if poly_upper_occupied_k == CHUNK_SIZE_U32 - 1 {
	cmpl $15, %esi
	jne .LBB226_166
	movq 984(%rsp), %rdx
		// crates/impact_voxel/src/object.rs:2929
		self.0[2][1] += 1;
	incq %rdx
	movq %rdx, 984(%rsp)
	movq %rdx, 912(%rsp)
		// crates/impact_voxel/src/object.rs:2930
		}
	jmp .LBB226_166
	.p2align	4
.LBB226_214:
	movq 960(%rsp), %r15
	movl 472(%rsp), %r11d
	movq 976(%rsp), %rbx
	movq 968(%rsp), %r14
	movl 488(%rsp), %ebp
		// crates/impact_voxel/src/object/extraction.rs:1079
		.all(|(&lower, &upper)| lower > upper);
	cmpl %r15d, 340(%rsp)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/iter/traits/iterator.rs:2494
		accum = f(accum, x)?;
	jbe .LBB226_257
	cmpl %r14d, %ebp
	jbe .LBB226_257
	cmpl %ebx, %r11d
	jbe .LBB226_257
	movq 880(%rsp), %rcx
	movq 616(%rsp), %rsi
	movq 1224(%rsp), %rdx
	movl $49, %eax
.LBB226_218:
		// crates/impact_voxel/src/lib.rs:253
		self.encoded > Self::VOID_LIMIT
	cmpb $101, -48(%rdx,%rax)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/iter/macros.rs:315
		if !f(x) {
	jl .LBB226_236
		// crates/impact_voxel/src/lib.rs:253
		self.encoded > Self::VOID_LIMIT
	cmpb $101, -45(%rdx,%rax)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/iter/macros.rs:315
		if !f(x) {
	jl .LBB226_236
		// crates/impact_voxel/src/lib.rs:253
		self.encoded > Self::VOID_LIMIT
	cmpb $101, -42(%rdx,%rax)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/iter/macros.rs:315
		if !f(x) {
	jl .LBB226_236
		// crates/impact_voxel/src/lib.rs:253
		self.encoded > Self::VOID_LIMIT
	cmpb $101, -39(%rdx,%rax)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/iter/macros.rs:315
		if !f(x) {
	jl .LBB226_236
		// crates/impact_voxel/src/lib.rs:253
		self.encoded > Self::VOID_LIMIT
	cmpb $101, -36(%rdx,%rax)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/iter/macros.rs:315
		if !f(x) {
	jl .LBB226_236
		// crates/impact_voxel/src/lib.rs:253
		self.encoded > Self::VOID_LIMIT
	cmpb $101, -33(%rdx,%rax)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/iter/macros.rs:315
		if !f(x) {
	jl .LBB226_236
		// crates/impact_voxel/src/lib.rs:253
		self.encoded > Self::VOID_LIMIT
	cmpb $101, -30(%rdx,%rax)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/iter/macros.rs:315
		if !f(x) {
	jl .LBB226_236
		// crates/impact_voxel/src/lib.rs:253
		self.encoded > Self::VOID_LIMIT
	cmpb $101, -27(%rdx,%rax)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/iter/macros.rs:315
		if !f(x) {
	jl .LBB226_236
		// crates/impact_voxel/src/lib.rs:253
		self.encoded > Self::VOID_LIMIT
	cmpb $101, -24(%rdx,%rax)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/iter/macros.rs:315
		if !f(x) {
	jl .LBB226_236
		// crates/impact_voxel/src/lib.rs:253
		self.encoded > Self::VOID_LIMIT
	cmpb $101, -21(%rdx,%rax)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/iter/macros.rs:315
		if !f(x) {
	jl .LBB226_236
		// crates/impact_voxel/src/lib.rs:253
		self.encoded > Self::VOID_LIMIT
	cmpb $101, -18(%rdx,%rax)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/iter/macros.rs:315
		if !f(x) {
	jl .LBB226_236
		// crates/impact_voxel/src/lib.rs:253
		self.encoded > Self::VOID_LIMIT
	cmpb $101, -15(%rdx,%rax)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/iter/macros.rs:315
		if !f(x) {
	jl .LBB226_236
		// crates/impact_voxel/src/lib.rs:253
		self.encoded > Self::VOID_LIMIT
	cmpb $101, -12(%rdx,%rax)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/iter/macros.rs:315
		if !f(x) {
	jl .LBB226_236
		// crates/impact_voxel/src/lib.rs:253
		self.encoded > Self::VOID_LIMIT
	cmpb $101, -9(%rdx,%rax)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/iter/macros.rs:315
		if !f(x) {
	jl .LBB226_236
		// crates/impact_voxel/src/lib.rs:253
		self.encoded > Self::VOID_LIMIT
	cmpb $101, -6(%rdx,%rax)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/iter/macros.rs:315
		if !f(x) {
	jl .LBB226_236
		// crates/impact_voxel/src/lib.rs:253
		self.encoded > Self::VOID_LIMIT
	cmpb $101, -3(%rdx,%rax)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/iter/macros.rs:315
		if !f(x) {
	jl .LBB226_236
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/non_null.rs:1714
		self.as_ptr() == other.as_ptr()
	cmpq $12289, %rax
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/iter/macros.rs:180
		if ptr == crate::intrinsics::transmute::<$ptr, NonNull<T>>(end_or_len) {
	je .LBB226_355
		// crates/impact_voxel/src/lib.rs:253
		self.encoded > Self::VOID_LIMIT
	cmpb $100, (%rdx,%rax)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/iter/macros.rs:315
		if !f(x) {
	leaq 51(%rax), %rax
	jg .LBB226_218
.LBB226_236:
	movq 656(%rsp), %rdi
	movq 648(%rsp), %r8
		// crates/impact_voxel/src/object.rs:2989
		if non_empty_count == full_face_count {
	xorl %eax, %eax
	movq 672(%rsp), %r10
	movq 664(%rsp), %r11
	movq 680(%rsp), %r14
	movq 640(%rsp), %rbx
	testq %rdi, %rdi
	setne %al
	xorl %edx, %edx
	testq %r8, %r8
	setne %dl
	shll $9, %edx
	cmpq $256, %r8
	movl $256, %r8d
	cmovel %r8d, %edx
	addl %eax, %eax
	cmpq $256, %rdi
	movl $1, %edi
	cmovel %edi, %eax
	xorl %r9d, %r9d
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/array/mod.rs:162
		}
	orl %edx, %eax
		// crates/impact_voxel/src/object.rs:2989
		if non_empty_count == full_face_count {
	testq %r10, %r10
	setne %r9b
	xorl %edx, %edx
	testq %r11, %r11
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/array/mod.rs:162
		}
	movzwl %ax, %eax
		// crates/impact_voxel/src/object.rs:2989
		if non_empty_count == full_face_count {
	setne %dl
	shll $9, %edx
	cmpq $256, %r11
	cmovel %r8d, %edx
	addl %r9d, %r9d
	cmpq $256, %r10
	cmovel %edi, %r9d
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/array/mod.rs:162
		}
	orl %edx, %r9d
		// crates/impact_voxel/src/object.rs:2989
		if non_empty_count == full_face_count {
	xorl %edx, %edx
	testq %r14, %r14
	setne %dl
	xorl %r10d, %r10d
	testq %rbx, %rbx
	setne %r10b
	shll $9, %r10d
	cmpq $256, %rbx
	cmovel %r8d, %r10d
	addl %edx, %edx
	cmpq $256, %r14
	cmovel %edi, %edx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/array/mod.rs:162
		}
	shll $16, %r9d
	orl %r10d, %edx
	movl %edx, %edi
	shlq $32, %rdx
	orq %rdx, %r9
		// crates/impact_voxel/src/object/extraction.rs:1089
		chunk.face_distributions =
	movw %di, 13(%rcx)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/array/mod.rs:162
		}
	orq %r9, %rax
		// crates/impact_voxel/src/object/extraction.rs:1089
		chunk.face_distributions =
	movl %eax, 9(%rcx)
		// crates/impact_voxel/src/object.rs:163
		bitflags! {
	orb $64, 8(%rcx)
	.cfi_escape 0x2e, 0x00
	movq 16(%rsp), %rax
	movq 600(%rsp), %rdi
	movq 608(%rsp), %rdx
	movq 8(%rsp), %r13
	movq 288(%rsp), %r8
	movq %rax, (%rsp)
		// crates/impact_voxel/src/object/extraction.rs:1096
		.update_local_connected_regions_for_chunk_with_single_region(
	callq *<impact_voxel::object::split_detection::SplitDetector>::update_local_connected_regions_for_chunk_with_single_region@GOTPCREL(%rip)
	jmp .LBB226_258
	.p2align	4
.LBB226_237:
	movq 88(%rsp), %rax
.LBB226_238:
		// crates/impact_voxel/src/object/extraction.rs:779
		match chunk {
	cmpb $2, (%rsp)
	je .LBB226_243
		// crates/impact_voxel/src/voxel_types.rs:136
		self.0 as usize
	movzbl %r12b, %edi
		// crates/impact_voxel/src/object/inertia.rs:709
		let density = voxel_type_densities[voxel_type.idx()];
	cmpq %rdi, 248(%rsp)
	jbe .LBB226_441
		// crates/impact_voxel/src/object/inertia.rs:713
		let xl = (chunk_indices[0] as f32) * chunk_extent;
	vcvtuqq2psx 208(%rsp), %xmm1
	vmovaps 1920(%rsp), %xmm2
	movq 264(%rsp), %rax
	movq 152(%rsp), %rcx
		// crates/impact_voxel/src/object/inertia.rs:709
		let density = voxel_type_densities[voxel_type.idx()];
	vmovss (%rax,%rdi,4), %xmm0
	movq 320(%rsp), %rax
		// crates/impact_voxel/src/object/inertia.rs:749
		((1.0 / 3.0) * chunk_extent_pow_2 * density) * (h3_sub_l3.yxx() + h3_sub_l3.zzy());
	vmulss 700(%rsp), %xmm0, %xmm7
		// crates/impact_voxel/src/object/inertia.rs:746
		let mass = chunk_extent_pow_3 * density;
	vmulss 708(%rsp), %xmm0, %xmm9
		// crates/impact_voxel/src/object/inertia.rs:713
		let xl = (chunk_indices[0] as f32) * chunk_extent;
	vmulps %xmm1, %xmm2, %xmm1
		// crates/impact_voxel/src/object/inertia.rs:714
		let xh = xl + chunk_extent;
	vaddps %xmm1, %xmm2, %xmm2
		// crates/impact_voxel/src/object/inertia.rs:715
		let xl2 = xl * xl;
	vmulps %xmm1, %xmm1, %xmm3
		// crates/impact_voxel/src/object/inertia.rs:716
		let xh2 = xh * xh;
	vmulps %xmm2, %xmm2, %xmm4
		// crates/impact_voxel/src/object/inertia.rs:717
		let xl3 = xl2 * xl;
	vmulps %xmm3, %xmm1, %xmm1
		// crates/impact_voxel/src/object/inertia.rs:718
		let xh3 = xh2 * xh;
	vmulps %xmm4, %xmm2, %xmm2
		// crates/impact_voxel/src/object/inertia.rs:720
		let xh3_sub_xl3 = xh3 - xl3;
	vsubps %xmm1, %xmm2, %xmm1
		// crates/impact_voxel/src/object/inertia.rs:719
		let xh2_sub_xl2 = xh2 - xl2;
	vsubps %xmm3, %xmm4, %xmm2
		// crates/impact_voxel/src/object/inertia.rs:731
		let zl = (chunk_indices[2] as f32) * chunk_extent;
	vcvtusi2ssq 224(%rsp), %xmm15, %xmm3
	vmovaps 1936(%rsp), %xmm4
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/../../stdarch/crates/core_arch/src/macros.rs:173
		$crate::intrinsics::simd::simd_shuffle(
	vshufps $1, %xmm1, %xmm1, %xmm8
		// crates/impact_voxel/src/object/inertia.rs:731
		let zl = (chunk_indices[2] as f32) * chunk_extent;
	vmulss %xmm3, %xmm4, %xmm3
		// crates/impact_voxel/src/object/inertia.rs:732
		let zh = zl + chunk_extent;
	vaddss %xmm3, %xmm4, %xmm4
		// crates/impact_voxel/src/object/inertia.rs:733
		let zl2 = zl * zl;
	vmulss %xmm3, %xmm3, %xmm5
		// crates/impact_voxel/src/object/inertia.rs:734
		let zh2 = zh * zh;
	vmulss %xmm4, %xmm4, %xmm6
		// crates/impact_voxel/src/object/inertia.rs:735
		let zl3 = zl2 * zl;
	vmulss %xmm5, %xmm3, %xmm3
		// crates/impact_voxel/src/object/inertia.rs:736
		let zh3 = zh2 * zh;
	vmulss %xmm6, %xmm4, %xmm4
		// crates/impact_voxel/src/object/inertia.rs:737
		let zh2_sub_zl2 = zh2 - zl2;
	vsubss %xmm5, %xmm6, %xmm5
		// crates/impact_voxel/src/object/inertia.rs:747
		let moments = (0.5 * chunk_extent_pow_2 * density) * h2_sub_l2;
	vmulss 704(%rsp), %xmm0, %xmm6
		// crates/impact_voxel/src/object/inertia.rs:751
		(0.25 * chunk_extent * density) * h2_sub_l2.component_mul(&h2_sub_l2.yzx());
	vmulss 696(%rsp), %xmm0, %xmm0
		// crates/impact_voxel/src/object/inertia.rs:738
		let zh3_sub_zl3 = zh3 - zl3;
	vsubss %xmm3, %xmm4, %xmm3
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/glam-0.30.10/src/f32/sse2/vec3a.rs:109
		unsafe { UnionCast { a: [x, y, z, z] }.v }
	vinsertps $32, %xmm5, %xmm2, %xmm4
	vshufps $4, %xmm5, %xmm2, %xmm2
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/../../stdarch/crates/core_arch/src/macros.rs:173
		$crate::intrinsics::simd::simd_shuffle(
	vshufps $16, %xmm1, %xmm3, %xmm1
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/../../stdarch/crates/core_arch/src/simd.rs:57
		unsafe { crate::intrinsics::simd::simd_splat(value) }
	vbroadcastss %xmm7, %xmm3
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/../../stdarch/crates/core_arch/src/x86/sse.rs:36
		unsafe { simd_add(a, b) }
	vaddps %xmm1, %xmm8, %xmm1
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/../../stdarch/crates/core_arch/src/simd.rs:57
		unsafe { crate::intrinsics::simd::simd_splat(value) }
	vbroadcastss %xmm6, %xmm6
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/../../stdarch/crates/core_arch/src/x86/sse.rs:88
		unsafe { simd_mul(a, b) }
	vmulps %xmm1, %xmm3, %xmm1
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/../../stdarch/crates/core_arch/src/macros.rs:173
		$crate::intrinsics::simd::simd_shuffle(
	vshufps $9, %xmm4, %xmm4, %xmm3
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/../../stdarch/crates/core_arch/src/simd.rs:57
		unsafe { crate::intrinsics::simd::simd_splat(value) }
	vbroadcastss %xmm0, %xmm0
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/../../stdarch/crates/core_arch/src/x86/sse.rs:88
		unsafe { simd_mul(a, b) }
	vmulps %xmm2, %xmm6, %xmm6
	vmulps %xmm3, %xmm2, %xmm2
	vmulps %xmm2, %xmm0, %xmm0
		// crates/impact_voxel/src/object/inertia.rs:463
		self.source.mass -= chunk_mass;
	vmovss 48(%rax), %xmm2
	vsubss %xmm9, %xmm2, %xmm2
	vmovss %xmm2, 48(%rax)
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/glam-0.30.10/src/f32/sse2/vec3a.rs:1733
		self.0 = unsafe { _mm_sub_ps(self.0, rhs.0) };
	vmovaps (%rax), %xmm2
	vmovaps 16(%rax), %xmm3
	vmovaps 32(%rax), %xmm4
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/../../stdarch/crates/core_arch/src/x86/sse.rs:62
		unsafe { simd_sub(a, b) }
	vsubps %xmm6, %xmm2, %xmm2
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/glam-0.30.10/src/f32/sse2/vec3a.rs:1733
		self.0 = unsafe { _mm_sub_ps(self.0, rhs.0) };
	vmovaps %xmm2, (%rax)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/../../stdarch/crates/core_arch/src/x86/sse.rs:62
		unsafe { simd_sub(a, b) }
	vsubps %xmm1, %xmm3, %xmm2
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/glam-0.30.10/src/f32/sse2/vec3a.rs:1733
		self.0 = unsafe { _mm_sub_ps(self.0, rhs.0) };
	vmovaps %xmm2, 16(%rax)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/../../stdarch/crates/core_arch/src/x86/sse.rs:62
		unsafe { simd_sub(a, b) }
	vsubps %xmm0, %xmm4, %xmm2
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/glam-0.30.10/src/f32/sse2/vec3a.rs:1733
		self.0 = unsafe { _mm_sub_ps(self.0, rhs.0) };
	vmovaps %xmm2, 32(%rax)
		// crates/impact_voxel/src/object/inertia.rs:468
		self.destination.mass += chunk_mass;
	vaddss 48(%rcx), %xmm9, %xmm2
	vmovss %xmm2, 48(%rcx)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/../../stdarch/crates/core_arch/src/x86/sse.rs:36
		unsafe { simd_add(a, b) }
	vaddps (%rcx), %xmm6, %xmm2
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/glam-0.30.10/src/f32/sse2/vec3a.rs:1609
		self.0 = unsafe { _mm_add_ps(self.0, rhs.0) };
	vmovaps %xmm2, (%rcx)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/../../stdarch/crates/core_arch/src/x86/sse.rs:36
		unsafe { simd_add(a, b) }
	vaddps 16(%rcx), %xmm1, %xmm1
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/glam-0.30.10/src/f32/sse2/vec3a.rs:1609
		self.0 = unsafe { _mm_add_ps(self.0, rhs.0) };
	vmovaps %xmm1, 16(%rcx)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/../../stdarch/crates/core_arch/src/x86/sse.rs:36
		unsafe { simd_add(a, b) }
	vaddps 32(%rcx), %xmm0, %xmm0
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/glam-0.30.10/src/f32/sse2/vec3a.rs:1609
		self.0 = unsafe { _mm_add_ps(self.0, rhs.0) };
	vmovaps %xmm0, 32(%rcx)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/vec/mod.rs:1037
		let len = self.len;
	movq 112(%rsp), %rbx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/vec/mod.rs:1040
		if len == self.buf.capacity() {
	cmpq 96(%rsp), %rbx
	jne .LBB226_242
	.cfi_escape 0x2e, 0x00
	movq 16(%rsp), %rax
	movq 8(%rsp), %r13
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/vec/mod.rs:1041
		self.buf.grow_one();
	leaq 96(%rsp), %rdi
	movq %rax, (%rsp)
	vzeroupper
	callq *<alloc::raw_vec::RawVec<impact_voxel::voxel_types::FixedVoxelMaterialProperties>>::grow_one@GOTPCREL(%rip)
.LBB226_242:
	movq 432(%rsp), %rcx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/raw_vec/mod.rs:614
		self.ptr.cast().as_non_null_ptr()
	movq 104(%rsp), %rdx
		// crates/impact_voxel/src/object/extraction.rs:843
		poly_chunks.push(VoxelChunk::Uniform(chunk));
	shlq $32, %r12
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mut_ptr.rs:961
		unsafe { intrinsics::offset(self, count) }
	movq %rbx, %rsi
	shlq $4, %rsi
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/vec/mod.rs:1046
		self.len = len + 1;
	incq %rbx
	movb $63, %r15b
		// crates/impact_voxel/src/object/extraction.rs:843
		poly_chunks.push(VoxelChunk::Uniform(chunk));
	movl %ecx, %eax
	orq %r12, %rax
	incq %rcx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mod.rs:1933
		intrinsics::write_via_move(dst, src)
	movq %rax, (%rdx,%rsi)
	movb $4, 9(%rdx,%rsi)
	movq 544(%rsp), %rdx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/vec/mod.rs:1046
		self.len = len + 1;
	movq %rbx, 112(%rsp)
	movq %rcx, 432(%rsp)
		// crates/impact_voxel/src/object/extraction.rs:844
		self.chunks[chunk_idx] = VoxelChunk::Void;
	movb $3, (%rdx)
		// crates/impact_voxel/src/object/extraction.rs:847
		}
	jmp .LBB226_323
	.p2align	4
.LBB226_243:
	movq 80(%rsp), %rdi
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/vec/mod.rs:1903
		&mut *core::intrinsics::aggregate_raw_ptr::<*mut [T], _, _>(self.as_mut_ptr(), self.len)
	movq 40(%rax), %rdx
		// crates/impact_voxel/src/object.rs:3149
		(data_offset as usize) << (3 * LOG2_CHUNK_SIZE)
	shlq $12, %rdi
		// crates/impact_voxel/src/object.rs:3166
		&mut voxels[start_voxel_idx..start_voxel_idx + CHUNK_VOXEL_COUNT]
	leaq 4096(%rdi), %rsi
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/index.rs:451
		&& self.end <= slice.len()
	cmpq %rdx, %rsi
	ja .LBB226_424
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/index.rs:101
		let ptr = unsafe { crate::intrinsics::offset(ptr, offset) };
	leaq (%rdi,%rdi,2), %rbx
	addq 32(%rax), %rbx
	.cfi_escape 0x2e, 0x00
	vmovaps 560(%rsp), %xmm0
	movq 16(%rsp), %rax
	movq 264(%rsp), %rcx
	movq 248(%rsp), %r8
	movq 8(%rsp), %r13
		// crates/impact_voxel/src/object/inertia.rs:433
		compute_moments_for_non_uniform_chunk(
	movl $4096, %edx
	leaq 1376(%rsp), %rdi
	leaq 208(%rsp), %r9
	movq %rbx, %rsi
	movq %rax, (%rsp)
	vzeroupper
	callq impact_voxel::object::inertia::compute_moments_for_non_uniform_chunk
	movq 320(%rsp), %rax
		// crates/impact_voxel/src/object/inertia.rs:432
		let (chunk_mass, chunk_moments, chunk_moments_of_inertia, chunk_products_of_inertia) =
	vmovss 1408(%rsp), %xmm0
	vmovaps 1376(%rsp), %xmm1
	vmovaps 1392(%rsp), %xmm2
	vmovaps 1424(%rsp), %xmm3
	movq 152(%rsp), %rcx
		// crates/impact_voxel/src/object/inertia.rs:440
		self.source.mass -= chunk_mass;
	vmovss 48(%rax), %xmm4
	vsubss %xmm0, %xmm4, %xmm4
	vmovss %xmm4, 48(%rax)
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/glam-0.30.10/src/f32/sse2/vec3a.rs:1733
		self.0 = unsafe { _mm_sub_ps(self.0, rhs.0) };
	vmovaps (%rax), %xmm4
	vmovaps 16(%rax), %xmm5
	vmovaps 32(%rax), %xmm6
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/../../stdarch/crates/core_arch/src/x86/sse.rs:62
		unsafe { simd_sub(a, b) }
	vsubps %xmm1, %xmm4, %xmm4
	vsubps %xmm2, %xmm5, %xmm7
	vsubps %xmm3, %xmm6, %xmm5
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/glam-0.30.10/src/f32/sse2/vec3a.rs:1733
		self.0 = unsafe { _mm_sub_ps(self.0, rhs.0) };
	vmovaps %xmm4, (%rax)
	vmovaps %xmm7, 16(%rax)
	vmovaps %xmm5, 32(%rax)
		// crates/impact_voxel/src/object/inertia.rs:445
		self.destination.mass += chunk_mass;
	vaddss 48(%rcx), %xmm0, %xmm0
	vmovss %xmm0, 48(%rcx)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/../../stdarch/crates/core_arch/src/x86/sse.rs:36
		unsafe { simd_add(a, b) }
	vaddps (%rcx), %xmm1, %xmm0
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/glam-0.30.10/src/f32/sse2/vec3a.rs:1609
		self.0 = unsafe { _mm_add_ps(self.0, rhs.0) };
	vmovaps %xmm0, (%rcx)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/../../stdarch/crates/core_arch/src/x86/sse.rs:36
		unsafe { simd_add(a, b) }
	vaddps 16(%rcx), %xmm2, %xmm0
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/glam-0.30.10/src/f32/sse2/vec3a.rs:1609
		self.0 = unsafe { _mm_add_ps(self.0, rhs.0) };
	vmovaps %xmm0, 16(%rcx)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/../../stdarch/crates/core_arch/src/x86/sse.rs:36
		unsafe { simd_add(a, b) }
	vaddps 32(%rcx), %xmm3, %xmm0
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/glam-0.30.10/src/f32/sse2/vec3a.rs:1609
		self.0 = unsafe { _mm_add_ps(self.0, rhs.0) };
	vmovaps %xmm0, 32(%rcx)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/raw_vec/mod.rs:619
		if elem_size == 0 { usize::MAX } else { self.cap.as_inner() }
	movq 352(%rsp), %rax
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/vec/mod.rs:1471
		self.buf.reserve(self.len, additional);
	movq 368(%rsp), %r14
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/num/uint_macros.rs:2621
		intrinsics::wrapping_sub(self, rhs)
	subq %r14, %rax
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/raw_vec/mod.rs:767
		additional > self.capacity(elem_layout.size()).wrapping_sub(len)
	cmpq $4095, %rax
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/raw_vec/mod.rs:673
		if self.needs_to_grow(len, additional, elem_layout) {
	jbe .LBB226_358
.LBB226_246:
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mut_ptr.rs:961
		unsafe { intrinsics::offset(self, count) }
	leaq (%r14,%r14,2), %rdi
	addq 360(%rsp), %rdi
	.cfi_escape 0x2e, 0x00
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mod.rs:551
		unsafe { crate::intrinsics::copy_nonoverlapping(src, dst, count) }
	movl $12288, %edx
	movq %rbx, %rsi
	callq *memcpy@GOTPCREL(%rip)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/vec/mod.rs:2939
		self.len += count;
	addq $4096, %r14
	movl $93, %eax
	movq %r14, 368(%rsp)
	.p2align	4
.LBB226_247:
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/specialize.rs:25
		*item = unsafe { ptr::read(&value) };
	movb $1, -91(%rbx,%rax)
	movw $32767, -93(%rbx,%rax)
	movb $1, -88(%rbx,%rax)
	movw $32767, -90(%rbx,%rax)
	movb $1, -85(%rbx,%rax)
	movw $32767, -87(%rbx,%rax)
	movb $1, -82(%rbx,%rax)
	movw $32767, -84(%rbx,%rax)
	movb $1, -79(%rbx,%rax)
	movw $32767, -81(%rbx,%rax)
	movb $1, -76(%rbx,%rax)
	movw $32767, -78(%rbx,%rax)
	movb $1, -73(%rbx,%rax)
	movw $32767, -75(%rbx,%rax)
	movb $1, -70(%rbx,%rax)
	movw $32767, -72(%rbx,%rax)
	movb $1, -67(%rbx,%rax)
	movw $32767, -69(%rbx,%rax)
	movb $1, -64(%rbx,%rax)
	movw $32767, -66(%rbx,%rax)
	movb $1, -61(%rbx,%rax)
	movw $32767, -63(%rbx,%rax)
	movb $1, -58(%rbx,%rax)
	movw $32767, -60(%rbx,%rax)
	movb $1, -55(%rbx,%rax)
	movw $32767, -57(%rbx,%rax)
	movb $1, -52(%rbx,%rax)
	movw $32767, -54(%rbx,%rax)
	movb $1, -49(%rbx,%rax)
	movw $32767, -51(%rbx,%rax)
	movb $1, -46(%rbx,%rax)
	movw $32767, -48(%rbx,%rax)
	movb $1, -43(%rbx,%rax)
	movw $32767, -45(%rbx,%rax)
	movb $1, -40(%rbx,%rax)
	movw $32767, -42(%rbx,%rax)
	movb $1, -37(%rbx,%rax)
	movw $32767, -39(%rbx,%rax)
	movb $1, -34(%rbx,%rax)
	movw $32767, -36(%rbx,%rax)
	movb $1, -31(%rbx,%rax)
	movw $32767, -33(%rbx,%rax)
	movb $1, -28(%rbx,%rax)
	movw $32767, -30(%rbx,%rax)
	movb $1, -25(%rbx,%rax)
	movw $32767, -27(%rbx,%rax)
	movb $1, -22(%rbx,%rax)
	movw $32767, -24(%rbx,%rax)
	movb $1, -19(%rbx,%rax)
	movw $32767, -21(%rbx,%rax)
	movb $1, -16(%rbx,%rax)
	movw $32767, -18(%rbx,%rax)
	movb $1, -13(%rbx,%rax)
	movw $32767, -15(%rbx,%rax)
	movb $1, -10(%rbx,%rax)
	movw $32767, -12(%rbx,%rax)
	movb $1, -7(%rbx,%rax)
	movw $32767, -9(%rbx,%rax)
	movb $1, -4(%rbx,%rax)
	movw $32767, -6(%rbx,%rax)
	movb $1, -1(%rbx,%rax)
	movw $32767, -3(%rbx,%rax)
	movb $1, 2(%rbx,%rax)
	movw $32767, (%rbx,%rax)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/non_null.rs:1714
		self.as_ptr() == other.as_ptr()
	addq $96, %rax
	cmpq $12381, %rax
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/iter/macros.rs:180
		if ptr == crate::intrinsics::transmute::<$ptr, NonNull<T>>(end_or_len) {
	jne .LBB226_247
	movq 544(%rsp), %rax
	movq 8(%rsp), %r13
		// crates/impact_voxel/src/object/extraction.rs:797
		self.chunks[chunk_idx] = VoxelChunk::Void;
	movb $3, (%rax)
		// crates/impact_voxel/src/object/extraction.rs:799
		let face_distributions = chunk.face_distributions;
	movl 416(%rsp), %ecx
	movzbl 420(%rsp), %eax
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/vec/mod.rs:3054
		let len = self.len;
	movq 112(%rsp), %r15
		// crates/impact_voxel/src/object/extraction.rs:799
		let face_distributions = chunk.face_distributions;
	movl %ecx, 1760(%rsp)
		// crates/impact_voxel/src/object/extraction.rs:801
		let poly_chunk = NonUniformVoxelChunk {
	movl %ecx, 424(%rsp)
		// crates/impact_voxel/src/object/extraction.rs:812
		non_uniform_chunks_inside.push((chunk, poly_chunks.len()));
	movl %ecx, 1376(%rsp)
	movzwl 420(%rsp), %ecx
		// crates/impact_voxel/src/object/extraction.rs:799
		let face_distributions = chunk.face_distributions;
	movb %al, 1764(%rsp)
		// crates/impact_voxel/src/object/extraction.rs:801
		let poly_chunk = NonUniformVoxelChunk {
	movb %al, 428(%rsp)
		// crates/impact_voxel/src/object/extraction.rs:812
		non_uniform_chunks_inside.push((chunk, poly_chunks.len()));
	movw %cx, 1380(%rsp)
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/allocator-api2-0.2.21/src/stable/vec/mod.rs:1793
		if self.len == self.buf.capacity() {
	cmpq %r13, 536(%rsp)
	jne .LBB226_274
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/num/uint_macros.rs:896
		if intrinsics::unlikely(intrinsics::add_with_overflow(self, rhs).1) {
	cmpq $-1, %r13
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/intrinsics/mod.rs:459
		if b {
	je .LBB226_425
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/num/uint_macros.rs:896
		if intrinsics::unlikely(intrinsics::add_with_overflow(self, rhs).1) {
	leaq 1(%r13), %rcx
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/allocator-api2-0.2.21/src/stable/raw_vec.rs:498
		let cap = cmp::max(self.cap * 2, required_cap);
	leaq (%r13,%r13), %rax
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:1038
		if other < self { self } else { other }
	movl $4, %r14d
	cmpq %rax, %rcx
	cmovaq %rcx, %rax
	cmpq $5, %rax
	cmovaeq %rax, %r14
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/alloc/layout.rs:573
		if element_size != 0 && n > Layout::max_size_for_alignment(alignment) / element_size {
	leaq (,%r14,8), %rcx
	leaq (%rcx,%rcx,2), %rcx
	movq %rcx, 48(%rsp)
	movabsq $384307168202282325, %rcx
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/allocator-api2-0.2.21/src/stable/raw_vec.rs:333
		if mem::size_of::<T>() == 0 || self.cap == 0 {
	testq %r13, %r13
	je .LBB226_279
	movq 8(%rsp), %r13
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/alloc/layout.rs:573
		if element_size != 0 && n > Layout::max_size_for_alignment(alignment) / element_size {
	cmpq %rcx, %rax
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:966
		match self {
	ja .LBB226_425
		// crates/impact_alloc/src/arena.rs:169
		unsafe { &*self.arena_ptr }
	movq 64(%rsp), %rdi
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/alloc/layout.rs:573
		if element_size != 0 && n > Layout::max_size_for_alignment(alignment) / element_size {
	leaq (,%r13,8), %rax
	leaq (%rax,%rax,2), %rcx
	movq 16(%rdi), %rax
	movq 32(%rax), %rbx
	cmpq 24(%rsp), %rbx
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:2355
		if align_is_compatible && self.is_last_allocation(ptr) {
	je .LBB226_293
	movq %rcx, %rbp
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:1898
		let start = footer.data.as_ptr();
	movq (%rax), %rcx
.LBB226_254:
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mut_ptr.rs:470
		unsafe { intrinsics::arith_offset(self, count) as *mut T }
	andq $-8, %rbx
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:1952
		if aligned_ptr < start || aligned_size > capacity {
	cmpq %rcx, %rbx
	jb .LBB226_287
	movq %rbx, %rdx
	subq %rcx, %rdx
	cmpq %rdx, 48(%rsp)
	ja .LBB226_287
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mut_ptr.rs:470
		unsafe { intrinsics::arith_offset(self, count) as *mut T }
	subq 48(%rsp), %rbx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/mem/mod.rs:931
		crate::intrinsics::write_via_move(dest, src);
	movq %rbx, 32(%rax)
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:1880
		if let Some(p) = self.try_alloc_layout_fast(layout) {
	jmp .LBB226_289
.LBB226_257:
	movq 656(%rsp), %rdx
	movq 648(%rsp), %rsi
		// crates/impact_voxel/src/object.rs:2989
		if non_empty_count == full_face_count {
	xorl %eax, %eax
	movq 664(%rsp), %r9
	movl $256, %r8d
	movl $1, %edi
	movq 640(%rsp), %r10
	testq %rdx, %rdx
	setne %al
	xorl %ecx, %ecx
	testq %rsi, %rsi
	setne %cl
	shll $9, %ecx
	cmpq $256, %rsi
	movq 672(%rsp), %rsi
	cmovel %r8d, %ecx
	addl %eax, %eax
	cmpq $256, %rdx
	cmovel %edi, %eax
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/array/mod.rs:162
		}
	orl %ecx, %eax
		// crates/impact_voxel/src/object.rs:2989
		if non_empty_count == full_face_count {
	xorl %ecx, %ecx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/array/mod.rs:162
		}
	movzwl %ax, %eax
		// crates/impact_voxel/src/object.rs:2989
		if non_empty_count == full_face_count {
	testq %rsi, %rsi
	setne %cl
	xorl %edx, %edx
	testq %r9, %r9
	setne %dl
	shll $9, %edx
	cmpq $256, %r9
	movq 680(%rsp), %r9
	cmovel %r8d, %edx
	addl %ecx, %ecx
	cmpq $256, %rsi
	cmovel %edi, %ecx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/array/mod.rs:162
		}
	orl %edx, %ecx
		// crates/impact_voxel/src/object.rs:2989
		if non_empty_count == full_face_count {
	xorl %edx, %edx
	testq %r9, %r9
	setne %dl
	xorl %esi, %esi
	testq %r10, %r10
	setne %sil
	shll $9, %esi
	cmpq $256, %r10
	cmovel %r8d, %esi
	addl %edx, %edx
	cmpq $256, %r9
	cmovel %edi, %edx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/array/mod.rs:162
		}
	shll $16, %ecx
	orl %esi, %edx
	movl %edx, %esi
	shlq $32, %rdx
	orq %rdx, %rcx
	movl $15, %edx
	orq %rcx, %rax
	movq 880(%rsp), %rcx
		// crates/impact_voxel/src/object/extraction.rs:1089
		chunk.face_distributions =
	movl %eax, 9(%rcx)
	movw %si, 13(%rcx)
		// crates/impact_voxel/src/object.rs:163
		bitflags! {
	andb $-65, 8(%rcx)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:1077
		if other < self { other } else { self }
	cmpl $15, %r15d
		// crates/impact_voxel/src/object/extraction.rs:1105
		lower_occupied_voxels[dim] as usize
	movl 340(%rsp), %eax
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:1077
		if other < self { other } else { self }
	cmovael %edx, %r15d
	incl %r15d
	cmpl $15, %r14d
	cmovael %edx, %r14d
	incl %r14d
	cmpl $15, %ebx
	cmovael %edx, %ebx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/array/mod.rs:114
		try_from_fn(NeverShortCircuit::wrap_mut_1(f)).0
	movq %rax, 1376(%rsp)
	movq %r15, 1384(%rsp)
		// crates/impact_voxel/src/object/extraction.rs:1105
		lower_occupied_voxels[dim] as usize
	movl %ebp, %eax
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/array/mod.rs:114
		try_from_fn(NeverShortCircuit::wrap_mut_1(f)).0
	movq %rax, 1392(%rsp)
		// crates/impact_voxel/src/object/extraction.rs:1105
		lower_occupied_voxels[dim] as usize
	movl %r11d, %eax
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/array/mod.rs:114
		try_from_fn(NeverShortCircuit::wrap_mut_1(f)).0
	movq %r14, 1400(%rsp)
	movq %rax, 1408(%rsp)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:1077
		if other < self { other } else { self }
	incl %ebx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/array/mod.rs:114
		try_from_fn(NeverShortCircuit::wrap_mut_1(f)).0
	movq %rbx, 1416(%rsp)
	.cfi_escape 0x2e, 0x00
	movq 16(%rsp), %rax
	movq 600(%rsp), %rdi
	movq 616(%rsp), %rsi
	movq 608(%rsp), %rdx
	movq 8(%rsp), %r13
		// crates/impact_voxel/src/object/extraction.rs:1109
		.update_local_connected_regions_within_occupied_ranges_for_chunk(
	leaq 1376(%rsp), %r9
	movq 288(%rsp), %r8
	movq %rax, (%rsp)
	callq *<impact_voxel::object::split_detection::SplitDetector>::update_local_connected_regions_within_occupied_ranges_for_chunk@GOTPCREL(%rip)
.LBB226_258:
	movq 1232(%rsp), %rcx
	movl 348(%rsp), %eax
	xorl %edi, %edi
		// crates/impact_voxel/src/object/extraction.rs:1121
		.all(|(&lower, &upper)| lower > upper);
	cmpl 464(%rsp), %eax
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/iter/traits/iterator.rs:2494
		accum = f(accum, x)?;
	jbe .LBB226_265
	movl 344(%rsp), %eax
	cmpl 456(%rsp), %eax
	jbe .LBB226_265
	movl 336(%rsp), %eax
	cmpl 448(%rsp), %eax
	jbe .LBB226_265
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/iter/macros.rs:314
		while let Some(x) = self.next() {
	subq 1208(%rsp), %rcx
	movq 1216(%rsp), %rax
	addq $12288, %rcx
	.p2align	4
.LBB226_262:
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/non_null.rs:1714
		self.as_ptr() == other.as_ptr()
	testq %rcx, %rcx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/iter/macros.rs:180
		if ptr == crate::intrinsics::transmute::<$ptr, NonNull<T>>(end_or_len) {
	je .LBB226_276
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/iter/macros.rs:315
		if !f(x) {
	addq $-3, %rcx
		// crates/impact_voxel/src/lib.rs:253
		self.encoded > Self::VOID_LIMIT
	cmpb $100, 1(%rax)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/non_null.rs:656
		unsafe { transmute(intrinsics::offset(self.as_ptr(), count)) }
	leaq 3(%rax), %rax
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/iter/macros.rs:315
		if !f(x) {
	jg .LBB226_262
	movb $64, %dil
.LBB226_265:
	movq 928(%rsp), %rax
	movq 920(%rsp), %rcx
		// crates/impact_voxel/src/object.rs:2989
		if non_empty_count == full_face_count {
	xorl %r15d, %r15d
	movq 944(%rsp), %rdx
	movq 936(%rsp), %rsi
	movq 952(%rsp), %r11
	movl %edi, 184(%rsp)
	testq %rax, %rax
	setne %r15b
	xorl %r8d, %r8d
	testq %rcx, %rcx
	setne %r8b
	shll $9, %r8d
	cmpq $256, %rcx
	movl $256, %ecx
	cmovel %ecx, %r8d
	addl %r15d, %r15d
	cmpq $256, %rax
	movl $1, %eax
	cmovel %eax, %r15d
	xorl %r14d, %r14d
	testq %rdx, %rdx
	movl %r8d, 200(%rsp)
	setne %r14b
	xorl %r9d, %r9d
	testq %rsi, %rsi
	setne %r9b
	shll $9, %r9d
	cmpq $256, %rsi
	movq 912(%rsp), %rsi
	cmovel %ecx, %r9d
	addl %r14d, %r14d
	cmpq $256, %rdx
	cmovel %eax, %r14d
	xorl %r12d, %r12d
	testq %r11, %r11
	movl %r9d, 256(%rsp)
	setne %r12b
	xorl %r10d, %r10d
	testq %rsi, %rsi
	setne %r10b
	shll $9, %r10d
	cmpq $256, %rsi
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/vec/mod.rs:3054
		let len = self.len;
	movq 112(%rsp), %rsi
		// crates/impact_voxel/src/object.rs:2989
		if non_empty_count == full_face_count {
	cmovel %ecx, %r10d
	movq 464(%rsp), %rcx
	addl %r12d, %r12d
	cmpq $256, %r11
	cmovel %eax, %r12d
	movl $15, %eax
	movl %r10d, 240(%rsp)
	movq %rsi, 80(%rsp)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:1077
		if other < self { other } else { self }
	cmpl $15, %ecx
	cmovael %eax, %ecx
	movq %rcx, 464(%rsp)
	movq 456(%rsp), %rcx
	cmpl $15, %ecx
	cmovael %eax, %ecx
	movq %rcx, 456(%rsp)
	movq 448(%rsp), %rcx
	cmpl $15, %ecx
	cmovael %eax, %ecx
	movq 16(%rsp), %rax
	movq %rcx, 448(%rsp)
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/allocator-api2-0.2.21/src/stable/vec/mod.rs:1793
		if self.len == self.buf.capacity() {
	cmpq %rax, 392(%rsp)
	jne .LBB226_275
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/num/uint_macros.rs:896
		if intrinsics::unlikely(intrinsics::add_with_overflow(self, rhs).1) {
	cmpq $-1, %rax
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/intrinsics/mod.rs:459
		if b {
	je .LBB226_426
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/num/uint_macros.rs:896
		if intrinsics::unlikely(intrinsics::add_with_overflow(self, rhs).1) {
	leaq 1(%rax), %rcx
	movq %rax, %r11
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/allocator-api2-0.2.21/src/stable/raw_vec.rs:498
		let cap = cmp::max(self.cap * 2, required_cap);
	addq %rax, %rax
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:1038
		if other < self { self } else { other }
	cmpq %rax, %rcx
	cmovaq %rcx, %rax
	movl $4, %ecx
	cmpq $5, %rax
	cmovaeq %rax, %rcx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/alloc/layout.rs:573
		if element_size != 0 && n > Layout::max_size_for_alignment(alignment) / element_size {
	imulq $56, %rcx, %rdx
	movq %rcx, 48(%rsp)
	movabsq $164703072086692425, %rcx
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/allocator-api2-0.2.21/src/stable/raw_vec.rs:333
		if mem::size_of::<T>() == 0 || self.cap == 0 {
	testq %r11, %r11
	je .LBB226_283
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/alloc/layout.rs:573
		if element_size != 0 && n > Layout::max_size_for_alignment(alignment) / element_size {
	cmpq %rcx, %rax
	movq 16(%rsp), %rax
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:966
		match self {
	ja .LBB226_426
		// crates/impact_alloc/src/arena.rs:169
		unsafe { &*self.arena_ptr }
	movq 64(%rsp), %rdi
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/alloc/layout.rs:573
		if element_size != 0 && n > Layout::max_size_for_alignment(alignment) / element_size {
	imulq $56, %rax, %rcx
	movq %rcx, 192(%rsp)
	movq 16(%rdi), %rax
	movq 32(%rax), %rbx
	cmpq 128(%rsp), %rbx
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:2355
		if align_is_compatible && self.is_last_allocation(ptr) {
	je .LBB226_297
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:1898
		let start = footer.data.as_ptr();
	movq (%rax), %rcx
.LBB226_271:
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mut_ptr.rs:470
		unsafe { intrinsics::arith_offset(self, count) as *mut T }
	andq $-8, %rbx
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:1952
		if aligned_ptr < start || aligned_size > capacity {
	cmpq %rcx, %rbx
	jb .LBB226_290
	movq %rbx, %rsi
	subq %rcx, %rsi
	cmpq %rsi, %rdx
	ja .LBB226_290
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mut_ptr.rs:470
		unsafe { intrinsics::arith_offset(self, count) as *mut T }
	subq %rdx, %rbx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/mem/mod.rs:931
		crate::intrinsics::write_via_move(dest, src);
	movq %rbx, 32(%rax)
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:1880
		if let Some(p) = self.try_alloc_layout_fast(layout) {
	jmp .LBB226_292
.LBB226_274:
	movq 24(%rsp), %rbx
	movq %r13, %r14
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/allocator-api2-0.2.21/src/stable/vec/mod.rs:1793
		if self.len == self.buf.capacity() {
	jmp .LBB226_302
.LBB226_275:
	movq 128(%rsp), %rbx
	movq %rax, 48(%rsp)
	jmp .LBB226_319
.LBB226_276:
	movq 624(%rsp), %rax
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/vec/mod.rs:1037
		let len = self.len;
	movq 112(%rsp), %rbx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/vec/mod.rs:1827
		self.len = len;
	movq %rax, 368(%rsp)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/vec/mod.rs:1040
		if len == self.buf.capacity() {
	cmpq 96(%rsp), %rbx
	jne .LBB226_278
	.cfi_escape 0x2e, 0x00
	movq 16(%rsp), %rax
	movq 8(%rsp), %r13
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/vec/mod.rs:1041
		self.buf.grow_one();
	leaq 96(%rsp), %rdi
	movq %rax, (%rsp)
	callq *<alloc::raw_vec::RawVec<impact_voxel::voxel_types::FixedVoxelMaterialProperties>>::grow_one@GOTPCREL(%rip)
.LBB226_278:
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/raw_vec/mod.rs:614
		self.ptr.cast().as_non_null_ptr()
	movq 104(%rsp), %rax
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mut_ptr.rs:961
		unsafe { intrinsics::offset(self, count) }
	movq %rbx, %rcx
	shlq $4, %rcx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/vec/mod.rs:1046
		self.len = len + 1;
	incq %rbx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mod.rs:1933
		intrinsics::write_via_move(dst, src)
	movb $3, 9(%rax,%rcx)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/vec/mod.rs:1046
		self.len = len + 1;
	movq %rbx, 112(%rsp)
	jmp .LBB226_322
.LBB226_279:
	movq 8(%rsp), %r13
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/alloc/layout.rs:573
		if element_size != 0 && n > Layout::max_size_for_alignment(alignment) / element_size {
	cmpq %rcx, %rax
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:966
		match self {
	ja .LBB226_425
		// crates/impact_alloc/src/arena.rs:169
		unsafe { &*self.arena_ptr }
	movq 64(%rsp), %rdi
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cell.rs:555
		unsafe { *self.value.get() }
	movq 16(%rdi), %rax
	movq 32(%rax), %rbx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mut_ptr.rs:470
		unsafe { intrinsics::arith_offset(self, count) as *mut T }
	andq $-8, %rbx
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:1952
		if aligned_ptr < start || aligned_size > capacity {
	movq %rbx, %rcx
	subq (%rax), %rcx
	setb %dl
	cmpq $96, %rcx
	setb %cl
	orb %dl, %cl
	je .LBB226_301
	.cfi_escape 0x2e, 0x00
	movq 16(%rsp), %rax
	movq 48(%rsp), %rdx
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:1883
		self.alloc_layout_slow(layout).ok_or(AllocErr)
	movl $8, %esi
	xorl %r13d, %r13d
	movq %rax, (%rsp)
	callq *<bumpalo::Bump>::alloc_layout_slow@GOTPCREL(%rip)
	movq %rax, %rbx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:966
		match self {
	testq %rax, %rax
	jne .LBB226_302
	jmp .LBB226_447
.LBB226_283:
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/alloc/layout.rs:573
		if element_size != 0 && n > Layout::max_size_for_alignment(alignment) / element_size {
	cmpq %rcx, %rax
	movq 16(%rsp), %rax
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:966
		match self {
	ja .LBB226_426
		// crates/impact_alloc/src/arena.rs:169
		unsafe { &*self.arena_ptr }
	movq 64(%rsp), %rdi
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cell.rs:555
		unsafe { *self.value.get() }
	movq 16(%rdi), %rax
	movq 32(%rax), %rbx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mut_ptr.rs:470
		unsafe { intrinsics::arith_offset(self, count) as *mut T }
	andq $-8, %rbx
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:1952
		if aligned_ptr < start || aligned_size > capacity {
	movq %rbx, %rcx
	subq (%rax), %rcx
	setb %sil
	cmpq $224, %rcx
	setb %cl
	orb %sil, %cl
	je .LBB226_318
	.cfi_escape 0x2e, 0x00
	movq 8(%rsp), %r13
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:1883
		self.alloc_layout_slow(layout).ok_or(AllocErr)
	movl $8, %esi
	movq $0, (%rsp)
	movq %rdx, %rbp
	callq *<bumpalo::Bump>::alloc_layout_slow@GOTPCREL(%rip)
	movq %rax, %rbx
	movq %rbp, %rdx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:966
		match self {
	testq %rax, %rax
	jne .LBB226_319
	jmp .LBB226_448
.LBB226_287:
	.cfi_escape 0x2e, 0x00
	movq 16(%rsp), %rax
	movq 48(%rsp), %rdx
	movq 8(%rsp), %r13
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:1883
		self.alloc_layout_slow(layout).ok_or(AllocErr)
	movl $8, %esi
	movq %rax, (%rsp)
	callq *<bumpalo::Bump>::alloc_layout_slow@GOTPCREL(%rip)
	movq %rax, %rbx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/option.rs:1335
		match self {
	testq %rax, %rax
	je .LBB226_447
.LBB226_289:
	.cfi_escape 0x2e, 0x00
	movq 24(%rsp), %rsi
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mod.rs:551
		unsafe { crate::intrinsics::copy_nonoverlapping(src, dst, count) }
	movq %rbx, %rdi
	movq %rbp, %rdx
	callq *memcpy@GOTPCREL(%rip)
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:2371
		}
	jmp .LBB226_302
.LBB226_290:
	.cfi_escape 0x2e, 0x00
	movq 16(%rsp), %rax
	movq 8(%rsp), %r13
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:1883
		self.alloc_layout_slow(layout).ok_or(AllocErr)
	movl $8, %esi
	movq %rdx, %rbp
	movq %rax, (%rsp)
	callq *<bumpalo::Bump>::alloc_layout_slow@GOTPCREL(%rip)
	movq %rax, %rbx
	movq %rbp, %rdx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/option.rs:1335
		match self {
	testq %rax, %rax
	je .LBB226_448
.LBB226_292:
	.cfi_escape 0x2e, 0x00
	movq 128(%rsp), %rsi
	movq 192(%rsp), %rdx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mod.rs:551
		unsafe { crate::intrinsics::copy_nonoverlapping(src, dst, count) }
	movq %rbx, %rdi
	callq *memcpy@GOTPCREL(%rip)
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:2371
		}
	jmp .LBB226_319
.LBB226_293:
	movq 48(%rsp), %rdx
	movabsq $9223372036854775793, %rsi
	movq %rcx, %rbp
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:2358
		let delta = new_size - old_size;
	subq %rcx, %rdx
	leaq 8(%rsi), %rcx
	cmpq %rcx, %rdx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/mem/alignment.rs:137
		if align.is_power_of_two() {
	jae .LBB226_447
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:1898
		let start = footer.data.as_ptr();
	movq (%rax), %rcx
	movq 24(%rsp), %rsi
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:453
		ptr.wrapping_sub(ptr as usize & (divisor - 1))
	andl $7, %esi
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mut_ptr.rs:470
		unsafe { intrinsics::arith_offset(self, count) as *mut T }
	movq %rbx, %r13
	subq %rsi, %r13
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:1952
		if aligned_ptr < start || aligned_size > capacity {
	movq %r13, %rsi
	subq %rcx, %rsi
	jb .LBB226_254
	cmpq %rsi, %rdx
	ja .LBB226_254
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mut_ptr.rs:470
		unsafe { intrinsics::arith_offset(self, count) as *mut T }
	subq %rdx, %r13
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/mem/mod.rs:931
		crate::intrinsics::write_via_move(dest, src);
	movq %r13, 32(%rax)
	.cfi_escape 0x2e, 0x00
	movq 24(%rsp), %rsi
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mod.rs:642
		crate::intrinsics::copy(src, dst, count)
	movq %r13, %rdi
	movq %rbp, %rdx
	callq *memmove@GOTPCREL(%rip)
	movq %r13, %rbx
	jmp .LBB226_302
.LBB226_297:
	movabsq $9223372036854775793, %rsi
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:2358
		let delta = new_size - old_size;
	movq %rdx, %r8
	subq %rcx, %r8
	leaq 8(%rsi), %rcx
	cmpq %rcx, %r8
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/mem/alignment.rs:137
		if align.is_power_of_two() {
	jae .LBB226_448
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:1898
		let start = footer.data.as_ptr();
	movq (%rax), %rcx
	movq 128(%rsp), %rsi
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:453
		ptr.wrapping_sub(ptr as usize & (divisor - 1))
	andl $7, %esi
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mut_ptr.rs:470
		unsafe { intrinsics::arith_offset(self, count) as *mut T }
	movq %rbx, %r13
	subq %rsi, %r13
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:1952
		if aligned_ptr < start || aligned_size > capacity {
	movq %r13, %rsi
	subq %rcx, %rsi
	jb .LBB226_271
	cmpq %rsi, %r8
	ja .LBB226_271
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mut_ptr.rs:470
		unsafe { intrinsics::arith_offset(self, count) as *mut T }
	subq %r8, %r13
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/mem/mod.rs:931
		crate::intrinsics::write_via_move(dest, src);
	movq %r13, 32(%rax)
	.cfi_escape 0x2e, 0x00
	movq 128(%rsp), %rsi
	movq 192(%rsp), %rdx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mod.rs:642
		crate::intrinsics::copy(src, dst, count)
	movq %r13, %rdi
	callq *memmove@GOTPCREL(%rip)
	movq %r13, %rbx
	jmp .LBB226_319
.LBB226_301:
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mut_ptr.rs:470
		unsafe { intrinsics::arith_offset(self, count) as *mut T }
	addq $-96, %rbx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/mem/mod.rs:931
		crate::intrinsics::write_via_move(dest, src);
	movq %rbx, 32(%rax)
	.p2align	4
.LBB226_302:
	movq 536(%rsp), %rax
	movq 80(%rsp), %rcx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mut_ptr.rs:961
		unsafe { intrinsics::offset(self, count) }
	leaq (%rax,%rax,2), %rax
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mod.rs:1933
		intrinsics::write_via_move(dst, src)
	movl %ecx, (%rbx,%rax,8)
	movzbl 200(%rsp), %ecx
	movl %r12d, 4(%rbx,%rax,8)
	movb %cl, 8(%rbx,%rax,8)
	movzbl 184(%rsp), %ecx
	movb %cl, 9(%rbx,%rax,8)
	movl 1376(%rsp), %ecx
	movl %ecx, 10(%rbx,%rax,8)
	movzwl 1380(%rsp), %ecx
	movw %cx, 14(%rbx,%rax,8)
	movq %r15, 16(%rbx,%rax,8)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/vec/mod.rs:1037
		let len = self.len;
	movq 112(%rsp), %r15
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/vec/mod.rs:1040
		if len == self.buf.capacity() {
	cmpq 96(%rsp), %r15
	jne .LBB226_304
	.cfi_escape 0x2e, 0x00
	movq 16(%rsp), %rax
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/vec/mod.rs:1041
		self.buf.grow_one();
	leaq 96(%rsp), %rdi
	movq %r14, %r13
	movq %rbx, 24(%rsp)
	movq %rax, (%rsp)
	callq *<alloc::raw_vec::RawVec<impact_voxel::voxel_types::FixedVoxelMaterialProperties>>::grow_one@GOTPCREL(%rip)
.LBB226_304:
	movzbl 200(%rsp), %edx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/raw_vec/mod.rs:614
		self.ptr.cast().as_non_null_ptr()
	movq 104(%rsp), %rax
	movq 272(%rsp), %rsi
	movzbl 184(%rsp), %edi
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mut_ptr.rs:961
		unsafe { intrinsics::offset(self, count) }
	movq %r15, %rcx
	shlq $4, %rcx
	incq 536(%rsp)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/vec/mod.rs:1046
		self.len = len + 1;
	incq %r15
	andb $64, %dl
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mod.rs:1933
		intrinsics::write_via_move(dst, src)
	movl %esi, (%rax,%rcx)
	movl $0, 4(%rax,%rcx)
		// crates/impact_voxel/src/object/extraction.rs:815
		poly_non_uniform_chunk_count += 1;
	incq %rsi
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mod.rs:1933
		intrinsics::write_via_move(dst, src)
	movb %dl, 8(%rax,%rcx)
	movb %dil, 9(%rax,%rcx)
	movq %rsi, 272(%rsp)
	movl 424(%rsp), %edx
	movl %edx, 10(%rax,%rcx)
	movzbl 428(%rsp), %edx
	movb %dl, 14(%rax,%rcx)
	leaq 1393(%rsp), %rcx
	movq 440(%rsp), %rdx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/vec/mod.rs:1046
		self.len = len + 1;
	movq %r15, 112(%rsp)
	xorl %r15d, %r15d
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/array/iter.rs:68
		let data: [MaybeUninit<T>; N] = unsafe { transmute_unchecked(self) };
	movzbl 1764(%rsp), %eax
	movb %al, 4(%rcx)
	movl 1760(%rsp), %eax
	movl %eax, (%rcx)
		// crates/impact_voxel/src/object/extraction.rs:818
		face_distributions.into_iter().enumerate()
	movq $3, 1384(%rsp)
	movb %dil, 1392(%rsp)
	xorl %eax, %eax
	xorl %ecx, %ecx
	.p2align	4
.LBB226_305:
	movq %rdx, 440(%rsp)
	movl $3, %edx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ops/index_range.rs:131
		if self.len() > 0 {
	cmpq $3, %rcx
	je .LBB226_308
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mod.rs:1733
		crate::intrinsics::read_via_copy(src)
	movzwl 1392(%rsp,%rcx,2), %edx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/num/uint_macros.rs:971
		intrinsics::unchecked_add(self, rhs)
	incq %rcx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/option.rs:2775
		match self {
	cmpb $-1, %dl
	jne .LBB226_312
	movq %rcx, %rdx
.LBB226_308:
	movl 332(%rsp), %esi
	movq %rdx, %rcx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/option.rs:2790
		None => None,
	orl $255, %esi
	movl %esi, %edx
	movl %edx, 332(%rsp)
		// crates/impact_voxel/src/object/extraction.rs:818
		face_distributions.into_iter().enumerate()
	testb %dl, %dl
	je .LBB226_313
.LBB226_309:
	movzbl %dl, %edx
	cmpl $255, %edx
	je .LBB226_316
	movq 440(%rsp), %rdx
		// crates/impact_voxel/src/object/extraction.rs:821
		invalidated_faces |= Faces::all_lower()[dim];
	movb $4, 1074(%rsp)
	movw $513, 1072(%rsp)
	cmpq $2, %rdx
	ja .LBB226_429
		// crates/impact_voxel/src/utils.rs:21
		bitflags! {
	orb 1072(%rsp,%rdx), %r15b
		// crates/impact_voxel/src/object.rs:150
		#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
	cmpw $256, 332(%rsp)
		// crates/impact_voxel/src/object/extraction.rs:823
		if !distributions[1].is_empty() {
	jb .LBB226_305
	jmp .LBB226_314
	.p2align	4
.LBB226_312:
	movq %rax, 440(%rsp)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/iter/adapters/enumerate.rs:82
		self.count += 1;
	incq %rax
	movl %edx, 332(%rsp)
		// crates/impact_voxel/src/object/extraction.rs:818
		face_distributions.into_iter().enumerate()
	testb %dl, %dl
	jne .LBB226_309
.LBB226_313:
	movq 440(%rsp), %rdx
		// crates/impact_voxel/src/object.rs:150
		#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
	cmpw $256, 332(%rsp)
		// crates/impact_voxel/src/object/extraction.rs:823
		if !distributions[1].is_empty() {
	jb .LBB226_305
.LBB226_314:
		// crates/impact_voxel/src/object/extraction.rs:824
		invalidated_faces |= Faces::all_upper()[dim];
	movb $32, 1074(%rsp)
	movw $4104, 1072(%rsp)
	cmpq $2, %rdx
	ja .LBB226_430
		// crates/impact_voxel/src/utils.rs:21
		bitflags! {
	orb 1072(%rsp,%rdx), %r15b
		// crates/impact_voxel/src/object/extraction.rs:823
		if !distributions[1].is_empty() {
	jmp .LBB226_305
	.p2align	4
.LBB226_316:
		// crates/impact_voxel/src/utils.rs:21
		bitflags! {
	testb %r15b, %r15b
		// crates/impact_voxel/src/object/extraction.rs:1159
		if !invalidated_faces.is_empty() {
	je .LBB226_341
	movq %r14, 8(%rsp)
	movq %rbx, 24(%rsp)
		// crates/impact_voxel/src/utils.rs:21
		bitflags! {
	testb $1, %r15b
		// crates/impact_voxel/src/object/extraction.rs:1167
		if invalidated_faces.contains(Faces::all_lower()[dim.idx()])
	jne .LBB226_323
	jmp .LBB226_325
.LBB226_318:
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mut_ptr.rs:470
		unsafe { intrinsics::arith_offset(self, count) as *mut T }
	addq $-224, %rbx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/mem/mod.rs:931
		crate::intrinsics::write_via_move(dest, src);
	movq %rbx, 32(%rax)
	.p2align	4
.LBB226_319:
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mut_ptr.rs:961
		unsafe { intrinsics::offset(self, count) }
	imulq $56, 392(%rsp), %rsi
	movq 464(%rsp), %r8
	movq 456(%rsp), %rdi
	movq 448(%rsp), %rbp
	movq 80(%rsp), %r9
	movl 348(%rsp), %eax
	movl 344(%rsp), %ecx
	movl 336(%rsp), %edx
	incl %r8d
	incl %edi
	incl %ebp
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mod.rs:1933
		intrinsics::write_via_move(dst, src)
	movq %r9, (%rbx,%rsi)
	movq %rax, 8(%rbx,%rsi)
	movq %r8, 16(%rbx,%rsi)
	movq %rcx, 24(%rbx,%rsi)
	movq %rdi, 32(%rbx,%rsi)
	movq %rdx, 40(%rbx,%rsi)
	movq %rbp, 48(%rbx,%rsi)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/vec/mod.rs:1037
		let len = self.len;
	movq 112(%rsp), %rbp
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/vec/mod.rs:1040
		if len == self.buf.capacity() {
	cmpq 96(%rsp), %rbp
	jne .LBB226_321
	.cfi_escape 0x2e, 0x00
	movq 48(%rsp), %rax
	movq 8(%rsp), %r13
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/vec/mod.rs:1041
		self.buf.grow_one();
	leaq 96(%rsp), %rdi
	movq %rbx, 128(%rsp)
	movq %rax, (%rsp)
	callq *<alloc::raw_vec::RawVec<impact_voxel::voxel_types::FixedVoxelMaterialProperties>>::grow_one@GOTPCREL(%rip)
.LBB226_321:
	addl 256(%rsp), %r14d
	addl 240(%rsp), %r12d
	addl 200(%rsp), %r15d
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/raw_vec/mod.rs:614
		self.ptr.cast().as_non_null_ptr()
	movq 104(%rsp), %rcx
	movq 272(%rsp), %rsi
	movl 184(%rsp), %edi
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mut_ptr.rs:961
		unsafe { intrinsics::offset(self, count) }
	movq %rbp, %rdx
	shlq $4, %rdx
	incq 392(%rsp)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/vec/mod.rs:1046
		self.len = len + 1;
	incq %rbp
	movq %rbx, 128(%rsp)
	shlq $32, %r12
	shll $16, %r14d
	movzwl %r15w, %eax
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mod.rs:1933
		intrinsics::write_via_move(dst, src)
	movl %esi, (%rcx,%rdx)
	movl $0, 4(%rcx,%rdx)
	movb %dil, 8(%rcx,%rdx)
		// crates/impact_voxel/src/object/extraction.rs:1153
		poly_non_uniform_chunk_count += 1;
	incq %rsi
	orq %r12, %r14
	movq %rsi, 272(%rsp)
	orq %r14, %rax
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mod.rs:1933
		intrinsics::write_via_move(dst, src)
	movl %eax, 9(%rcx,%rdx)
	shrq $32, %rax
	movw %ax, 13(%rcx,%rdx)
	movq 48(%rsp), %rcx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/vec/mod.rs:1046
		self.len = len + 1;
	movq %rbp, 112(%rsp)
	movq %rcx, 16(%rsp)
.LBB226_322:
	movq 88(%rsp), %rax
	movb $63, %r15b
	movq 208(%rax), %rcx
	movq 216(%rax), %rax
	movq %rcx, 192(%rsp)
	movq %rax, 784(%rsp)
.LBB226_323:
		// crates/impact_voxel/src/object/extraction.rs:1168
		&& chunk_indices[dim.idx()] > 0
	movq 208(%rsp), %rax
	testq %rax, %rax
	je .LBB226_326
	movq 216(%rsp), %rsi
		// crates/impact_voxel/src/object/extraction.rs:1171
		lower_chunk_indices[dim.idx()] -= 1;
	decq %rax
		// crates/impact_voxel/src/object.rs:1804
		chunk_indices[0] * self.chunk_idx_strides[0]
	imulq 192(%rsp), %rax
		// crates/impact_voxel/src/object.rs:1805
		+ chunk_indices[1] * self.chunk_idx_strides[1]
	imulq 784(%rsp), %rsi
		// crates/impact_voxel/src/object.rs:1804
		chunk_indices[0] * self.chunk_idx_strides[0]
	addq %rax, %rsi
	addq 224(%rsp), %rsi
	.cfi_escape 0x2e, 0x00
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hashbrown-0.16.1/src/set.rs:1093
		self.map.insert(value, ()).is_none()
	leaq 1120(%rsp), %rdi
	xorl %edx, %edx
	vzeroupper
	callq <hashbrown::map::HashMap<(usize, impact_voxel::utils::Dimension), (), rustc_hash::FxBuildHasher, &impact_alloc::arena::PoolArena>>::insert
	movq 8(%rsp), %r14
	movq 24(%rsp), %rbx
.LBB226_325:
		// crates/impact_voxel/src/utils.rs:21
		bitflags! {
	testb $8, %r15b
		// crates/impact_voxel/src/object/extraction.rs:1175
		if invalidated_faces.contains(Faces::all_upper()[dim.idx()])
	jne .LBB226_327
	jmp .LBB226_329
	.p2align	4
.LBB226_326:
	movq 8(%rsp), %r14
	movq 24(%rsp), %rbx
		// crates/impact_voxel/src/utils.rs:21
		bitflags! {
	testb $8, %r15b
		// crates/impact_voxel/src/object/extraction.rs:1175
		if invalidated_faces.contains(Faces::all_upper()[dim.idx()])
	je .LBB226_329
.LBB226_327:
	movq 88(%rsp), %rax
		// crates/impact_voxel/src/object/extraction.rs:1176
		&& chunk_indices[dim.idx()] < self.chunk_counts[dim.idx()] - 1
	movq 184(%rax), %rax
	decq %rax
	cmpq %rax, 208(%rsp)
	jae .LBB226_329
	.cfi_escape 0x2e, 0x00
	movq 288(%rsp), %rsi
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hashbrown-0.16.1/src/set.rs:1093
		self.map.insert(value, ()).is_none()
	leaq 1120(%rsp), %rdi
	xorl %edx, %edx
	movq %r14, 8(%rsp)
	movq %rbx, 24(%rsp)
	vzeroupper
	callq <hashbrown::map::HashMap<(usize, impact_voxel::utils::Dimension), (), rustc_hash::FxBuildHasher, &impact_alloc::arena::PoolArena>>::insert
.LBB226_329:
	movq 216(%rsp), %r12
		// crates/impact_voxel/src/utils.rs:21
		bitflags! {
	testb $2, %r15b
		// crates/impact_voxel/src/object/extraction.rs:1167
		if invalidated_faces.contains(Faces::all_lower()[dim.idx()])
	je .LBB226_332
	testq %r12, %r12
	je .LBB226_332
	movq 208(%rsp), %rax
		// crates/impact_voxel/src/object/extraction.rs:1171
		lower_chunk_indices[dim.idx()] -= 1;
	leaq -1(%r12), %rsi
		// crates/impact_voxel/src/object.rs:1805
		+ chunk_indices[1] * self.chunk_idx_strides[1]
	imulq 784(%rsp), %rsi
		// crates/impact_voxel/src/object.rs:1804
		chunk_indices[0] * self.chunk_idx_strides[0]
	imulq 192(%rsp), %rax
	addq %rax, %rsi
	addq 224(%rsp), %rsi
	.cfi_escape 0x2e, 0x00
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hashbrown-0.16.1/src/set.rs:1093
		self.map.insert(value, ()).is_none()
	leaq 1120(%rsp), %rdi
	movl $1, %edx
	movq %r14, 8(%rsp)
	movq %rbx, 24(%rsp)
	vzeroupper
	callq <hashbrown::map::HashMap<(usize, impact_voxel::utils::Dimension), (), rustc_hash::FxBuildHasher, &impact_alloc::arena::PoolArena>>::insert
.LBB226_332:
		// crates/impact_voxel/src/utils.rs:21
		bitflags! {
	testb $16, %r15b
		// crates/impact_voxel/src/object/extraction.rs:1175
		if invalidated_faces.contains(Faces::all_upper()[dim.idx()])
	je .LBB226_335
	movq 88(%rsp), %rax
		// crates/impact_voxel/src/object/extraction.rs:1176
		&& chunk_indices[dim.idx()] < self.chunk_counts[dim.idx()] - 1
	movq 192(%rax), %rax
	decq %rax
	cmpq %rax, %r12
	jae .LBB226_335
	.cfi_escape 0x2e, 0x00
	movq 288(%rsp), %rsi
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hashbrown-0.16.1/src/set.rs:1093
		self.map.insert(value, ()).is_none()
	leaq 1120(%rsp), %rdi
	movl $1, %edx
	movq %r14, 8(%rsp)
	movq %rbx, 24(%rsp)
	vzeroupper
	callq <hashbrown::map::HashMap<(usize, impact_voxel::utils::Dimension), (), rustc_hash::FxBuildHasher, &impact_alloc::arena::PoolArena>>::insert
.LBB226_335:
	movq 224(%rsp), %r13
		// crates/impact_voxel/src/utils.rs:21
		bitflags! {
	testb $4, %r15b
		// crates/impact_voxel/src/object/extraction.rs:1167
		if invalidated_faces.contains(Faces::all_lower()[dim.idx()])
	je .LBB226_338
	testq %r13, %r13
	je .LBB226_338
	movq 192(%rsp), %rax
		// crates/impact_voxel/src/object.rs:1805
		+ chunk_indices[1] * self.chunk_idx_strides[1]
	imulq 784(%rsp), %r12
		// crates/impact_voxel/src/object.rs:1804
		chunk_indices[0] * self.chunk_idx_strides[0]
	imulq 208(%rsp), %rax
	addq %r13, %r12
	leaq -1(%rax,%r12), %rsi
	.cfi_escape 0x2e, 0x00
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hashbrown-0.16.1/src/set.rs:1093
		self.map.insert(value, ()).is_none()
	leaq 1120(%rsp), %rdi
	movl $2, %edx
	movq %r14, 8(%rsp)
	movq %rbx, 24(%rsp)
	vzeroupper
	callq <hashbrown::map::HashMap<(usize, impact_voxel::utils::Dimension), (), rustc_hash::FxBuildHasher, &impact_alloc::arena::PoolArena>>::insert
.LBB226_338:
		// crates/impact_voxel/src/utils.rs:21
		bitflags! {
	testb $32, %r15b
		// crates/impact_voxel/src/object/extraction.rs:1175
		if invalidated_faces.contains(Faces::all_upper()[dim.idx()])
	je .LBB226_341
	movq 88(%rsp), %rax
		// crates/impact_voxel/src/object/extraction.rs:1176
		&& chunk_indices[dim.idx()] < self.chunk_counts[dim.idx()] - 1
	movq 200(%rax), %rax
	decq %rax
	cmpq %rax, %r13
	jae .LBB226_341
	.cfi_escape 0x2e, 0x00
	movq 288(%rsp), %rsi
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hashbrown-0.16.1/src/set.rs:1093
		self.map.insert(value, ()).is_none()
	leaq 1120(%rsp), %rdi
	movl $2, %edx
	movq %r14, 8(%rsp)
	movq %rbx, 24(%rsp)
	vzeroupper
	callq <hashbrown::map::HashMap<(usize, impact_voxel::utils::Dimension), (), rustc_hash::FxBuildHasher, &impact_alloc::arena::PoolArena>>::insert
	movq 16(%rsp), %rax
	movq %r14, %r13
	movq %rax, (%rsp)
	jmp .LBB226_342
	.p2align	4
.LBB226_341:
	movq 16(%rsp), %rax
	movq %rbx, 24(%rsp)
	movq %r14, %r13
	movq %rax, (%rsp)
.LBB226_342:
	.cfi_escape 0x2e, 0x00
	movq 280(%rsp), %rdi
	leaq 208(%rsp), %rsi
	vzeroupper
	callq <hashbrown::map::HashMap<[usize; 3], (), rustc_hash::FxBuildHasher>>::insert
		// crates/impact_voxel/src/object/extraction.rs:1188
		if chunk_indices[dim] > 0 {
	movq 208(%rsp), %rbx
	movq 88(%rsp), %r14
	testq %rbx, %rbx
	je .LBB226_345
		// crates/impact_voxel/src/object/extraction.rs:1189
		let mut neighbor_chunk_indices = chunk_indices;
	vmovaps 208(%rsp), %xmm0
	movq 224(%rsp), %rax
	movq %rax, 736(%rsp)
	vmovaps %xmm0, 720(%rsp)
		// crates/impact_voxel/src/object/extraction.rs:1190
		neighbor_chunk_indices[dim] -= 1;
	decq 720(%rsp)
	.cfi_escape 0x2e, 0x00
	movq 280(%rsp), %rdi
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hashbrown-0.16.1/src/set.rs:1093
		self.map.insert(value, ()).is_none()
	leaq 720(%rsp), %rsi
	callq <hashbrown::map::HashMap<[usize; 3], (), rustc_hash::FxBuildHasher>>::insert
.LBB226_345:
		// crates/impact_voxel/src/object/extraction.rs:1194
		if chunk_indices[dim] < self.chunk_counts[dim] - 1 {
	movq 184(%r14), %rax
	decq %rax
	cmpq %rax, %rbx
	jae .LBB226_347
		// crates/impact_voxel/src/object/extraction.rs:1195
		let mut neighbor_chunk_indices = chunk_indices;
	vmovaps 208(%rsp), %xmm0
	movq 224(%rsp), %rax
	movq %rax, 768(%rsp)
	vmovaps %xmm0, 752(%rsp)
		// crates/impact_voxel/src/object/extraction.rs:1196
		neighbor_chunk_indices[dim] += 1;
	incq 752(%rsp)
	.cfi_escape 0x2e, 0x00
	movq 280(%rsp), %rdi
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hashbrown-0.16.1/src/set.rs:1093
		self.map.insert(value, ()).is_none()
	leaq 752(%rsp), %rsi
	callq <hashbrown::map::HashMap<[usize; 3], (), rustc_hash::FxBuildHasher>>::insert
.LBB226_347:
		// crates/impact_voxel/src/object/extraction.rs:1188
		if chunk_indices[dim] > 0 {
	movq 216(%rsp), %rbx
	testq %rbx, %rbx
	je .LBB226_349
		// crates/impact_voxel/src/object/extraction.rs:1189
		let mut neighbor_chunk_indices = chunk_indices;
	vmovaps 208(%rsp), %xmm0
	movq 224(%rsp), %rax
	movq %rax, 736(%rsp)
	vmovaps %xmm0, 720(%rsp)
		// crates/impact_voxel/src/object/extraction.rs:1190
		neighbor_chunk_indices[dim] -= 1;
	decq 728(%rsp)
	.cfi_escape 0x2e, 0x00
	movq 280(%rsp), %rdi
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hashbrown-0.16.1/src/set.rs:1093
		self.map.insert(value, ()).is_none()
	leaq 720(%rsp), %rsi
	callq <hashbrown::map::HashMap<[usize; 3], (), rustc_hash::FxBuildHasher>>::insert
.LBB226_349:
		// crates/impact_voxel/src/object/extraction.rs:1194
		if chunk_indices[dim] < self.chunk_counts[dim] - 1 {
	movq 192(%r14), %rax
	decq %rax
	cmpq %rax, %rbx
	jae .LBB226_351
		// crates/impact_voxel/src/object/extraction.rs:1195
		let mut neighbor_chunk_indices = chunk_indices;
	vmovaps 208(%rsp), %xmm0
	movq 224(%rsp), %rax
	movq %rax, 768(%rsp)
	vmovaps %xmm0, 752(%rsp)
		// crates/impact_voxel/src/object/extraction.rs:1196
		neighbor_chunk_indices[dim] += 1;
	incq 760(%rsp)
	.cfi_escape 0x2e, 0x00
	movq 280(%rsp), %rdi
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hashbrown-0.16.1/src/set.rs:1093
		self.map.insert(value, ()).is_none()
	leaq 752(%rsp), %rsi
	callq <hashbrown::map::HashMap<[usize; 3], (), rustc_hash::FxBuildHasher>>::insert
.LBB226_351:
		// crates/impact_voxel/src/object/extraction.rs:1188
		if chunk_indices[dim] > 0 {
	movq 224(%rsp), %rbx
	testq %rbx, %rbx
	je .LBB226_353
		// crates/impact_voxel/src/object/extraction.rs:1189
		let mut neighbor_chunk_indices = chunk_indices;
	vmovaps 208(%rsp), %xmm0
	movq 224(%rsp), %rax
		// crates/impact_voxel/src/object/extraction.rs:1190
		neighbor_chunk_indices[dim] -= 1;
	decq %rax
		// crates/impact_voxel/src/object/extraction.rs:1189
		let mut neighbor_chunk_indices = chunk_indices;
	vmovaps %xmm0, 720(%rsp)
		// crates/impact_voxel/src/object/extraction.rs:1190
		neighbor_chunk_indices[dim] -= 1;
	movq %rax, 736(%rsp)
	.cfi_escape 0x2e, 0x00
	movq 280(%rsp), %rdi
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hashbrown-0.16.1/src/set.rs:1093
		self.map.insert(value, ()).is_none()
	leaq 720(%rsp), %rsi
	callq <hashbrown::map::HashMap<[usize; 3], (), rustc_hash::FxBuildHasher>>::insert
.LBB226_353:
		// crates/impact_voxel/src/object/extraction.rs:1194
		if chunk_indices[dim] < self.chunk_counts[dim] - 1 {
	movq 200(%r14), %rax
	decq %rax
	cmpq %rax, %rbx
	jae .LBB226_109
		// crates/impact_voxel/src/object/extraction.rs:1195
		let mut neighbor_chunk_indices = chunk_indices;
	vmovaps 208(%rsp), %xmm0
	movq 224(%rsp), %rax
		// crates/impact_voxel/src/object/extraction.rs:1196
		neighbor_chunk_indices[dim] += 1;
	incq %rax
		// crates/impact_voxel/src/object/extraction.rs:1195
		let mut neighbor_chunk_indices = chunk_indices;
	vmovaps %xmm0, 752(%rsp)
		// crates/impact_voxel/src/object/extraction.rs:1196
		neighbor_chunk_indices[dim] += 1;
	movq %rax, 768(%rsp)
	.cfi_escape 0x2e, 0x00
	movq 280(%rsp), %rdi
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hashbrown-0.16.1/src/set.rs:1093
		self.map.insert(value, ()).is_none()
	leaq 752(%rsp), %rsi
	callq <hashbrown::map::HashMap<[usize; 3], (), rustc_hash::FxBuildHasher>>::insert
	jmp .LBB226_109
.LBB226_355:
		// crates/impact_voxel/src/object/extraction.rs:1087
		self.chunks[chunk_idx] = VoxelChunk::Void;
	movb $3, 9(%rcx)
	jmp .LBB226_258
.LBB226_356:
	.cfi_escape 0x2e, 0x00
	movq 16(%rsp), %rax
	movq 8(%rsp), %r13
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/raw_vec/mod.rs:675
		do_reserve_and_handle(self, len, additional, elem_layout);
	movl $4096, %edx
	movl $1, %ecx
	movl $3, %r8d
	leaq 352(%rsp), %rdi
	movq %r12, %rsi
	movq %rax, (%rsp)
	vzeroupper
	callq <alloc::raw_vec::RawVecInner<_>>::reserve::do_reserve_and_handle::<alloc::alloc::Global>
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/vec/mod.rs:3054
		let len = self.len;
	movq 368(%rsp), %rbx
	movq 624(%rsp), %r12
	movq 616(%rsp), %r13
	jmp .LBB226_161
.LBB226_358:
	.cfi_escape 0x2e, 0x00
	movq 16(%rsp), %rax
	movq 8(%rsp), %r13
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/raw_vec/mod.rs:675
		do_reserve_and_handle(self, len, additional, elem_layout);
	movl $4096, %edx
	movl $1, %ecx
	movl $3, %r8d
	leaq 352(%rsp), %rdi
	movq %r14, %rsi
	movq %rax, (%rsp)
	callq <alloc::raw_vec::RawVecInner<_>>::reserve::do_reserve_and_handle::<alloc::alloc::Global>
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/vec/mod.rs:3054
		let len = self.len;
	movq 368(%rsp), %r14
	jmp .LBB226_246
.LBB226_360:
	.cfi_escape 0x2e, 0x00
	movq 16(%rsp), %rax
	movq 1168(%rsp), %rdi
	movq 8(%rsp), %r13
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/raw_vec/mod.rs:675
		do_reserve_and_handle(self, len, additional, elem_layout);
	movl $4096, %edx
	movl $1, %ecx
	movl $3, %r8d
	movq %rbx, %rsi
	movq %rax, (%rsp)
	vzeroupper
	callq <alloc::raw_vec::RawVecInner<_>>::reserve::do_reserve_and_handle::<alloc::alloc::Global>
	movq 88(%rsp), %rdi
	leaq 9(%r12), %r8
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/vec/mod.rs:3054
		let len = self.len;
	movq 40(%rdi), %rax
	jmp .LBB226_153
.LBB226_362:
	movq 1184(%rsp), %rax
	movq 432(%rsp), %rdx
	movq 272(%rsp), %rcx
	movq 392(%rsp), %rsi
	movq 1176(%rsp), %rdi
	movq 536(%rsp), %rbx
	movq %rax, 408(%rsp)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:1916
		fn lt(&self, other: &Self) -> bool { *self <  *other }
	cmpq 592(%rsp), %rax
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/iter/range.rs:900
		if self.start < self.end {
	jne .LBB226_105
	jmp .LBB226_365
.LBB226_363:
	xorl %esi, %esi
.LBB226_364:
	movq 32(%rsp), %rax
	movq %r13, (%rsp)
	xorl %ebx, %ebx
	xorl %ecx, %ecx
	xorl %edx, %edx
	movq %rax, 40(%rsp)
.LBB226_365:
	movq 840(%rsp), %rax
	movq 808(%rsp), %r8
	movq 848(%rsp), %r9
	xorl %ebp, %ebp
	movq %rsi, 392(%rsp)
		// crates/impact_voxel/src/object/extraction.rs:1206
		poly_split_detector_buffers,
	movq %rax, 1376(%rsp)
	movq %r8, 1384(%rsp)
	movq 816(%rsp), %r8
	movq $0, 1392(%rsp)
	movq %r9, 1400(%rsp)
	movq 856(%rsp), %r9
	movq %r8, 1408(%rsp)
	movq 824(%rsp), %r8
	movq $0, 1416(%rsp)
	movq %r9, 1424(%rsp)
	movq 864(%rsp), %r9
	movq %r8, 1432(%rsp)
	movq 832(%rsp), %r8
	movq $0, 1440(%rsp)
	movq %r9, 1448(%rsp)
	movq %r8, 1456(%rsp)
	movq $0, 1464(%rsp)
	.cfi_escape 0x2e, 0x00
	leaq 1760(%rsp), %rdi
	leaq 1376(%rsp), %rsi
	movq %rdx, 432(%rsp)
	movq %rcx, 272(%rsp)
		// crates/impact_voxel/src/object/extraction.rs:1205
		let mut poly_split_detector = SplitDetector::new(
	vzeroupper
	callq *<impact_voxel::object::split_detection::SplitDetector>::new@GOTPCREL(%rip)
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/allocator-api2-0.2.21/src/stable/vec/into_iter.rs:103
		if self.ptr == self.end {
	testq %rbx, %rbx
	je .LBB226_373
	movq 88(%rsp), %rax
	movq 104(%rsp), %r12
	movq 112(%rsp), %r14
	shlq $3, %rbx
	leaq 1386(%rsp), %r15
	xorl %ebp, %ebp
	leaq (%rbx,%rbx,2), %rbx
	addq $48, %rax
	movq %rax, 48(%rsp)
	jmp .LBB226_369
	.p2align	4
.LBB226_368:
	addq $24, %rbp
	cmpq %rbp, %rbx
	je .LBB226_373
.LBB226_369:
	movq 24(%rsp), %rsi
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mod.rs:1733
		crate::intrinsics::read_via_copy(src)
	movzwl 14(%rsi,%rbp), %ecx
	movzbl 9(%rsi,%rbp), %eax
	movw %cx, 1076(%rsp)
	movl 10(%rsi,%rbp), %ecx
	movl %ecx, 1072(%rsp)
	movq 16(%rsi,%rbp), %rdx
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/allocator-api2-0.2.21/src/stable/vec/into_iter.rs:117
		Some(unsafe { ptr::read(old) })
	movq (%rsi,%rbp), %rcx
	movzbl 8(%rsi,%rbp), %esi
	movb %sil, 2088(%rsp)
	movq %rcx, 2080(%rsp)
		// crates/impact_voxel/src/object/extraction.rs:1211
		for (chunk, poly_chunk_idx) in non_uniform_chunks_inside {
	cmpb $-1, %al
	je .LBB226_373
	movzbl 2088(%rsp), %ecx
	movzwl 1076(%rsp), %esi
	movb %cl, 1384(%rsp)
	movq 2080(%rsp), %rcx
	movq %rcx, 1376(%rsp)
	movl 1072(%rsp), %ecx
	movb %al, 1385(%rsp)
	movw %si, 4(%r15)
	movl %ecx, (%r15)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/index.rs:278
		&mut (*slice)[self]
	cmpq %r14, %rdx
	jae .LBB226_445
	movq %rdx, %rsi
	shlq $4, %rsi
		// crates/impact_voxel/src/object/extraction.rs:1215
		if let VoxelChunk::NonUniform(poly_chunk) = poly_chunk {
	cmpb $3, 9(%r12,%rsi)
	jae .LBB226_368
	addq %r12, %rsi
	.cfi_escape 0x2e, 0x00
	movq 48(%rsp), %rcx
		// crates/impact_voxel/src/object/extraction.rs:1216
		poly_split_detector.copy_local_connected_regions_from_chunk_in_other(
	leaq 1760(%rsp), %rdi
	leaq 1376(%rsp), %r8
	callq *<impact_voxel::object::split_detection::SplitDetector>::copy_local_connected_regions_from_chunk_in_other@GOTPCREL(%rip)
	jmp .LBB226_368
.LBB226_373:
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/allocator-api2-0.2.21/src/stable/raw_vec.rs:333
		if mem::size_of::<T>() == 0 || self.cap == 0 {
	testq %r13, %r13
	je .LBB226_375
		// crates/impact_alloc/src/arena.rs:169
		unsafe { &*self.arena_ptr }
	movq 64(%rsp), %rax
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:2505
		Bump::<MIN_ALIGN>::dealloc(self, ptr, layout)
	movq 16(%rax), %rax
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cell.rs:555
		unsafe { *self.value.get() }
	movq 32(%rax), %rcx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/non_null.rs:1714
		self.as_ptr() == other.as_ptr()
	cmpq 24(%rsp), %rcx
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:2231
		if self.is_last_allocation(ptr) {
	je .LBB226_381
.LBB226_375:
	movq 392(%rsp), %rax
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/allocator-api2-0.2.21/src/stable/vec/into_iter.rs:103
		if self.ptr == self.end {
	testq %rax, %rax
	je .LBB226_382
.LBB226_376:
	movq 128(%rsp), %r15
	imulq $56, %rax, %rbp
	movq 360(%rsp), %rax
	movq 104(%rsp), %rbx
	movq 112(%rsp), %r14
	movq 368(%rsp), %r12
	addq %r15, %rbp
	movq %rax, 48(%rsp)
	jmp .LBB226_378
	.p2align	4
.LBB226_377:
	addq $56, %r15
	cmpq %rbp, %r15
	je .LBB226_382
.LBB226_378:
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/allocator-api2-0.2.21/src/stable/vec/into_iter.rs:117
		Some(unsafe { ptr::read(old) })
	vmovdqu 8(%r15), %ymm0
	vmovdqu 40(%r15), %xmm1
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mod.rs:1733
		crate::intrinsics::read_via_copy(src)
	movq (%r15), %r8
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/allocator-api2-0.2.21/src/stable/vec/into_iter.rs:117
		Some(unsafe { ptr::read(old) })
	vmovdqa %xmm1, 1104(%rsp)
	vmovdqu %ymm0, 1072(%rsp)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/index.rs:278
		&mut (*slice)[self]
	cmpq %r14, %r8
	jae .LBB226_439
	movq %r8, %rcx
	shlq $4, %rcx
		// crates/impact_voxel/src/object/extraction.rs:1227
		if let VoxelChunk::NonUniform(poly_chunk) = poly_chunk {
	cmpb $3, 9(%rbx,%rcx)
	jae .LBB226_377
	addq %rbx, %rcx
	.cfi_escape 0x2e, 0x00
	movq 48(%rsp), %rsi
		// crates/impact_voxel/src/object/extraction.rs:1229
		.update_local_connected_regions_within_occupied_ranges_for_chunk(
	leaq 1760(%rsp), %rdi
	leaq 1072(%rsp), %r9
	movq %r12, %rdx
	vzeroupper
	callq *<impact_voxel::object::split_detection::SplitDetector>::update_local_connected_regions_within_occupied_ranges_for_chunk@GOTPCREL(%rip)
	jmp .LBB226_377
.LBB226_381:
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/alloc/layout.rs:573
		if element_size != 0 && n > Layout::max_size_for_alignment(alignment) / element_size {
	leaq (%r13,%r13,2), %rdx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mut_ptr.rs:961
		unsafe { intrinsics::offset(self, count) }
	leaq (%rcx,%rdx,8), %rcx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/mem/mod.rs:931
		crate::intrinsics::write_via_move(dest, src);
	movq %rcx, 32(%rax)
	movq 392(%rsp), %rax
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/allocator-api2-0.2.21/src/stable/vec/into_iter.rs:103
		if self.ptr == self.end {
	testq %rax, %rax
	jne .LBB226_376
.LBB226_382:
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/allocator-api2-0.2.21/src/stable/raw_vec.rs:333
		if mem::size_of::<T>() == 0 || self.cap == 0 {
	cmpq $0, (%rsp)
	je .LBB226_385
		// crates/impact_alloc/src/arena.rs:169
		unsafe { &*self.arena_ptr }
	movq 64(%rsp), %rax
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:2505
		Bump::<MIN_ALIGN>::dealloc(self, ptr, layout)
	movq 16(%rax), %rax
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cell.rs:555
		unsafe { *self.value.get() }
	movq 32(%rax), %rcx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/non_null.rs:1714
		self.as_ptr() == other.as_ptr()
	cmpq 128(%rsp), %rcx
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:2231
		if self.is_last_allocation(ptr) {
	jne .LBB226_385
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/alloc/layout.rs:573
		if element_size != 0 && n > Layout::max_size_for_alignment(alignment) / element_size {
	imulq $56, (%rsp), %rdx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mut_ptr.rs:961
		unsafe { intrinsics::offset(self, count) }
	addq %rdx, %rcx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/mem/mod.rs:931
		crate::intrinsics::write_via_move(dest, src);
	movq %rcx, 32(%rax)
.LBB226_385:
	.cfi_escape 0x2e, 0x00
	movq 88(%rsp), %r14
		// crates/impact_voxel/src/object/extraction.rs:1240
		self.update_occupied_ranges();
	movq %r14, %rdi
	vzeroupper
	callq *<impact_voxel::object::VoxelObject>::update_occupied_ranges@GOTPCREL(%rip)
	movq 576(%rsp), %rbx
	.cfi_escape 0x2e, 0x00
	leaq 1120(%rsp), %rsi
		// crates/impact_voxel/src/object/extraction.rs:1242
		self.update_upper_boundary_adjacencies_along_dim_for_chunks(invalidated_upper_face_chunks);
	movq %r14, %rdi
	callq <impact_voxel::object::VoxelObject>::update_upper_boundary_adjacencies_along_dim_for_chunks::<hashbrown::set::HashSet<(usize, impact_voxel::utils::Dimension), rustc_hash::FxBuildHasher, &impact_alloc::arena::PoolArena>>
	.cfi_escape 0x2e, 0x00
		// crates/impact_voxel/src/object/extraction.rs:1246
		self.resolve_connected_regions_between_all_chunks();
	movq %r14, %rdi
	callq *<impact_voxel::object::VoxelObject>::resolve_connected_regions_between_all_chunks@GOTPCREL(%rip)
	vmovdqu 1344(%rsp), %ymm2
	movq 800(%rsp), %rax
	vmovdqa 1872(%rsp), %xmm4
		// crates/impact_voxel/src/object/extraction.rs:1248
		let extraction_result = Self::complete_extracted_voxel_object(
	vmovdqa .LCPI226_20(%rip), %ymm3
		// crates/impact_voxel/src/object/extraction.rs:1258
		poly_invalidated_mesh_chunk_indices,
	vmovaps 1296(%rsp), %xmm1
		// crates/impact_voxel/src/object/extraction.rs:1249
		self.voxel_extent,
	vmovss 352(%r14), %xmm0
	movq 584(%rsp), %r15
		// crates/impact_voxel/src/object/extraction.rs:1250
		&self.origin_offset_in_root,
	addq $328, %r14
		// crates/impact_voxel/src/object/extraction.rs:1258
		poly_invalidated_mesh_chunk_indices,
	movq %rax, 2048(%rsp)
	movq 872(%rsp), %rax
	movq %r15, 2056(%rsp)
		// crates/impact_voxel/src/object/extraction.rs:1248
		let extraction_result = Self::complete_extracted_voxel_object(
	vpermi2q 1312(%rsp), %ymm2, %ymm3
	vmovdqa %xmm4, 2016(%rsp)
		// crates/impact_voxel/src/object/extraction.rs:1258
		poly_invalidated_mesh_chunk_indices,
	vmovups %xmm1, 2064(%rsp)
		// crates/impact_voxel/src/object/extraction.rs:1248
		let extraction_result = Self::complete_extracted_voxel_object(
	movq %rax, 2032(%rsp)
	movq 296(%rsp), %rax
	vmovdqu %ymm3, 1376(%rsp)
	movq %rax, 1408(%rsp)
	movq 304(%rsp), %rax
	movq %rax, 1416(%rsp)
	.cfi_escape 0x2e, 0x20
	movq 432(%rsp), %r8
	movq 272(%rsp), %r9
	leaq 2048(%rsp), %rax
	leaq 2016(%rsp), %rdx
	leaq 96(%rsp), %r10
	leaq 352(%rsp), %r11
	leaq 2080(%rsp), %rdi
	leaq 1376(%rsp), %rcx
	movq %r14, %rsi
	pushq %rax
	.cfi_adjust_cfa_offset 8
	leaq 1768(%rsp), %rax
	pushq %rax
	.cfi_adjust_cfa_offset 8
	pushq %r10
	.cfi_adjust_cfa_offset 8
	pushq %r11
	.cfi_adjust_cfa_offset 8
	vzeroupper
	callq <impact_voxel::object::VoxelObject>::complete_extracted_voxel_object
	addq $32, %rsp
	.cfi_adjust_cfa_offset -32
		// crates/impact_voxel/src/object/extraction.rs:1261
		let mut extracted = match extraction_result {
	cmpq $-1, 2080(%rsp)
	je .LBB226_404
		// crates/impact_voxel/src/object/extraction.rs:1262
		ExtractionResult::Extracted(extracted) => extracted,
	vmovups 2336(%rsp), %zmm1
	vmovups 2400(%rsp), %zmm0
	vmovdqu64 2080(%rsp), %zmm4
	vmovups 2208(%rsp), %zmm2
	vmovdqu64 2272(%rsp), %zmm3
	vmovups %zmm1, 1632(%rsp)
	vmovups 2144(%rsp), %zmm1
	vmovups %zmm0, 1696(%rsp)
	vmovdqu64 %zmm3, 1568(%rsp)
	vmovups %zmm2, 1504(%rsp)
	vmovdqu64 %zmm4, 1376(%rsp)
	vmovups %zmm1, 1440(%rsp)
	.cfi_escape 0x2e, 0x00
	leaq 1376(%rsp), %rdi
		// crates/impact_voxel/src/object/extraction.rs:1274
		.update_all_chunk_boundary_adjacencies();
	vzeroupper
	callq *<impact_voxel::object::VoxelObject>::update_all_chunk_boundary_adjacencies@GOTPCREL(%rip)
	.cfi_escape 0x2e, 0x00
	leaq 1376(%rsp), %rdi
		// crates/impact_voxel/src/object/extraction.rs:1278
		.resolve_connected_regions_between_all_chunks();
	callq *<impact_voxel::object::VoxelObject>::resolve_connected_regions_between_all_chunks@GOTPCREL(%rip)
	.cfi_escape 0x2e, 0x00
	leaq 1376(%rsp), %rdi
		// crates/impact_voxel/src/object/extraction.rs:1281
		extracted.voxel_object.update_occupied_ranges();
	callq *<impact_voxel::object::VoxelObject>::update_occupied_ranges@GOTPCREL(%rip)
		// crates/impact_voxel/src/object/extraction.rs:1283
		ExtractionResult::Extracted(extracted)
	vmovups 1632(%rsp), %zmm1
	vmovdqu64 1696(%rsp), %zmm0
	vmovdqu64 1376(%rsp), %zmm4
	vmovdqu64 1504(%rsp), %zmm2
	vmovdqu64 1568(%rsp), %zmm3
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/allocator-api2-0.2.21/src/stable/raw_vec.rs:333
		if mem::size_of::<T>() == 0 || self.cap == 0 {
	cmpq $0, 40(%rsp)
		// crates/impact_voxel/src/object/extraction.rs:1283
		ExtractionResult::Extracted(extracted)
	vmovups %zmm1, 256(%rbx)
	vmovdqu64 1440(%rsp), %zmm1
	vmovdqu64 %zmm0, 320(%rbx)
	vmovdqu64 %zmm3, 192(%rbx)
	vmovdqu64 %zmm2, 128(%rbx)
	vmovdqu64 %zmm4, (%rbx)
	vmovdqu64 %zmm1, 64(%rbx)
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/allocator-api2-0.2.21/src/stable/raw_vec.rs:333
		if mem::size_of::<T>() == 0 || self.cap == 0 {
	je .LBB226_396
		// crates/impact_alloc/src/arena.rs:169
		unsafe { &*self.arena_ptr }
	movq 64(%rsp), %rax
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:2505
		Bump::<MIN_ALIGN>::dealloc(self, ptr, layout)
	movq 16(%rax), %rax
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cell.rs:555
		unsafe { *self.value.get() }
	movq 32(%rax), %rcx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/non_null.rs:1714
		self.as_ptr() == other.as_ptr()
	cmpq 176(%rsp), %rcx
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:2231
		if self.is_last_allocation(ptr) {
	jne .LBB226_396
	movq 40(%rsp), %rdx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/alloc/layout.rs:573
		if element_size != 0 && n > Layout::max_size_for_alignment(alignment) / element_size {
	shlq $4, %rdx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mut_ptr.rs:961
		unsafe { intrinsics::offset(self, count) }
	addq %rdx, %rcx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/mem/mod.rs:931
		crate::intrinsics::write_via_move(dest, src);
	movq %rcx, 32(%rax)
.LBB226_396:
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/allocator-api2-0.2.21/src/stable/raw_vec.rs:333
		if mem::size_of::<T>() == 0 || self.cap == 0 {
	cmpq $0, 136(%rsp)
	je .LBB226_399
		// crates/impact_alloc/src/arena.rs:169
		unsafe { &*self.arena_ptr }
	movq 64(%rsp), %rax
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:2505
		Bump::<MIN_ALIGN>::dealloc(self, ptr, layout)
	movq 16(%rax), %rax
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cell.rs:555
		unsafe { *self.value.get() }
	movq 32(%rax), %rcx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/non_null.rs:1714
		self.as_ptr() == other.as_ptr()
	cmpq 168(%rsp), %rcx
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:2231
		if self.is_last_allocation(ptr) {
	jne .LBB226_399
	movq 136(%rsp), %rdx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/alloc/layout.rs:573
		if element_size != 0 && n > Layout::max_size_for_alignment(alignment) / element_size {
	shlq $5, %rdx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mut_ptr.rs:961
		unsafe { intrinsics::offset(self, count) }
	addq %rdx, %rcx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/mem/mod.rs:931
		crate::intrinsics::write_via_move(dest, src);
	movq %rcx, 32(%rax)
.LBB226_399:
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/allocator-api2-0.2.21/src/stable/raw_vec.rs:333
		if mem::size_of::<T>() == 0 || self.cap == 0 {
	cmpq $0, 144(%rsp)
	je .LBB226_402
		// crates/impact_alloc/src/arena.rs:169
		unsafe { &*self.arena_ptr }
	movq 64(%rsp), %rax
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:2505
		Bump::<MIN_ALIGN>::dealloc(self, ptr, layout)
	movq 16(%rax), %rax
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cell.rs:555
		unsafe { *self.value.get() }
	movq 32(%rax), %rcx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/non_null.rs:1714
		self.as_ptr() == other.as_ptr()
	cmpq 160(%rsp), %rcx
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:2231
		if self.is_last_allocation(ptr) {
	jne .LBB226_402
	movq 144(%rsp), %rdx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/alloc/layout.rs:573
		if element_size != 0 && n > Layout::max_size_for_alignment(alignment) / element_size {
	shlq $5, %rdx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mut_ptr.rs:961
		unsafe { intrinsics::offset(self, count) }
	addq %rdx, %rcx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/mem/mod.rs:931
		crate::intrinsics::write_via_move(dest, src);
	movq %rcx, 32(%rax)
.LBB226_402:
		// crates/impact_voxel/src/object/extraction.rs:1284
		}
	movq 72(%rsp), %rdi
	xorl %ebp, %ebp
	.cfi_escape 0x2e, 0x00
	vzeroupper
	callq core::ptr::drop_glue::<impact_alloc::arena::PoolArena>
	jmp .LBB226_414
.LBB226_404:
		// crates/impact_voxel/src/object/extraction.rs:1263
		result @ ExtractionResult::NotExtracted(_) => {
	vmovups 2336(%rsp), %zmm1
	vmovdqu64 2400(%rsp), %zmm0
	vmovdqu64 2080(%rsp), %zmm4
	vmovdqu64 2208(%rsp), %zmm2
	vmovdqu64 2272(%rsp), %zmm3
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/allocator-api2-0.2.21/src/stable/raw_vec.rs:333
		if mem::size_of::<T>() == 0 || self.cap == 0 {
	cmpq $0, 40(%rsp)
		// crates/impact_voxel/src/object/extraction.rs:1263
		result @ ExtractionResult::NotExtracted(_) => {
	vmovups %zmm1, 256(%rbx)
	vmovdqu64 2144(%rsp), %zmm1
	vmovdqu64 %zmm0, 320(%rbx)
	vmovdqu64 %zmm3, 192(%rbx)
	vmovdqu64 %zmm2, 128(%rbx)
	vmovdqu64 %zmm4, (%rbx)
	vmovdqu64 %zmm1, 64(%rbx)
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/allocator-api2-0.2.21/src/stable/raw_vec.rs:333
		if mem::size_of::<T>() == 0 || self.cap == 0 {
	je .LBB226_407
		// crates/impact_alloc/src/arena.rs:169
		unsafe { &*self.arena_ptr }
	movq 64(%rsp), %rax
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:2505
		Bump::<MIN_ALIGN>::dealloc(self, ptr, layout)
	movq 16(%rax), %rax
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cell.rs:555
		unsafe { *self.value.get() }
	movq 32(%rax), %rcx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/non_null.rs:1714
		self.as_ptr() == other.as_ptr()
	cmpq 176(%rsp), %rcx
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:2231
		if self.is_last_allocation(ptr) {
	jne .LBB226_407
	movq 40(%rsp), %rdx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/alloc/layout.rs:573
		if element_size != 0 && n > Layout::max_size_for_alignment(alignment) / element_size {
	shlq $4, %rdx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mut_ptr.rs:961
		unsafe { intrinsics::offset(self, count) }
	addq %rdx, %rcx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/mem/mod.rs:931
		crate::intrinsics::write_via_move(dest, src);
	movq %rcx, 32(%rax)
.LBB226_407:
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/allocator-api2-0.2.21/src/stable/raw_vec.rs:333
		if mem::size_of::<T>() == 0 || self.cap == 0 {
	cmpq $0, 136(%rsp)
	je .LBB226_410
		// crates/impact_alloc/src/arena.rs:169
		unsafe { &*self.arena_ptr }
	movq 64(%rsp), %rax
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:2505
		Bump::<MIN_ALIGN>::dealloc(self, ptr, layout)
	movq 16(%rax), %rax
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cell.rs:555
		unsafe { *self.value.get() }
	movq 32(%rax), %rcx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/non_null.rs:1714
		self.as_ptr() == other.as_ptr()
	cmpq 168(%rsp), %rcx
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:2231
		if self.is_last_allocation(ptr) {
	jne .LBB226_410
	movq 136(%rsp), %rdx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/alloc/layout.rs:573
		if element_size != 0 && n > Layout::max_size_for_alignment(alignment) / element_size {
	shlq $5, %rdx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mut_ptr.rs:961
		unsafe { intrinsics::offset(self, count) }
	addq %rdx, %rcx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/mem/mod.rs:931
		crate::intrinsics::write_via_move(dest, src);
	movq %rcx, 32(%rax)
.LBB226_410:
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/allocator-api2-0.2.21/src/stable/raw_vec.rs:333
		if mem::size_of::<T>() == 0 || self.cap == 0 {
	cmpq $0, 144(%rsp)
	je .LBB226_413
		// crates/impact_alloc/src/arena.rs:169
		unsafe { &*self.arena_ptr }
	movq 64(%rsp), %rax
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:2505
		Bump::<MIN_ALIGN>::dealloc(self, ptr, layout)
	movq 16(%rax), %rax
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cell.rs:555
		unsafe { *self.value.get() }
	movq 32(%rax), %rcx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/non_null.rs:1714
		self.as_ptr() == other.as_ptr()
	cmpq 160(%rsp), %rcx
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:2231
		if self.is_last_allocation(ptr) {
	jne .LBB226_413
	movq 144(%rsp), %rdx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/alloc/layout.rs:573
		if element_size != 0 && n > Layout::max_size_for_alignment(alignment) / element_size {
	shlq $5, %rdx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mut_ptr.rs:961
		unsafe { intrinsics::offset(self, count) }
	addq %rdx, %rcx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/mem/mod.rs:931
		crate::intrinsics::write_via_move(dest, src);
	movq %rcx, 32(%rax)
.LBB226_413:
		// crates/impact_voxel/src/object/extraction.rs:1284
		}
	movq 72(%rsp), %rdi
	xorl %ebp, %ebp
	.cfi_escape 0x2e, 0x00
	vzeroupper
	callq core::ptr::drop_glue::<impact_alloc::arena::PoolArena>
.LBB226_414:
	movq 576(%rsp), %rdi
.LBB226_415:
		// crates/impact_voxel/src/object/extraction.rs:631
		}
	movq %rdi, %rax
	addq $2472, %rsp
	.cfi_def_cfa_offset 56
	popq %rbx
	.cfi_def_cfa_offset 48
	popq %r12
	.cfi_def_cfa_offset 40
	popq %r13
	.cfi_def_cfa_offset 32
	popq %r14
	.cfi_def_cfa_offset 24
	popq %r15
	.cfi_def_cfa_offset 16
	popq %rbp
	.cfi_def_cfa_offset 8
	vzeroupper
	retq
.LBB226_416:
	.cfi_def_cfa_offset 2528
	movq 248(%rsp), %rsi
	movq %r9, %rdi
	leaq .Lanon.5bbf1cbe9e9f837e6488bd4d315a6b9f.641(%rip), %rdx
.LBB226_417:
	movb $1, %bpl
	.cfi_escape 0x2e, 0x00
	movq 16(%rsp), %rax
	movq 8(%rsp), %r13
	movq %rax, (%rsp)
	vzeroupper
	callq *core::panicking::panic_bounds_check@GOTPCREL(%rip)
	jmp .LBB226_452
.LBB226_418:
	movb $1, %bpl
	.cfi_escape 0x2e, 0x00
	movq 16(%rsp), %rax
	movq 8(%rsp), %r13
	movq %rax, (%rsp)
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/allocator-api2-0.2.21/src/stable/raw_vec.rs:612
		Err(CapacityOverflow) => capacity_overflow(),
	vzeroupper
	callq *allocator_api2::stable::raw_vec::capacity_overflow@GOTPCREL(%rip)
	jmp .LBB226_452
.LBB226_419:
	movb $1, %bpl
	.cfi_escape 0x2e, 0x00
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/index.rs:272
		&(*slice)[self]
	leaq .Lanon.5bbf1cbe9e9f837e6488bd4d315a6b9f.35(%rip), %rdx
	vzeroupper
	callq *core::panicking::panic_bounds_check@GOTPCREL(%rip)
	jmp .LBB226_452
.LBB226_420:
	movb $1, %bpl
	.cfi_escape 0x2e, 0x00
	movq 16(%rsp), %rax
	movq 8(%rsp), %r13
		// crates/impact_voxel/src/object/extraction.rs:859
		unreachable!();
	leaq .Lanon.5bbf1cbe9e9f837e6488bd4d315a6b9f.16(%rip), %rdi
	leaq .Lanon.5bbf1cbe9e9f837e6488bd4d315a6b9f.31(%rip), %rdx
	movl $40, %esi
	movq %rax, (%rsp)
	vzeroupper
	callq *core::panicking::panic@GOTPCREL(%rip)
	jmp .LBB226_452
.LBB226_421:
	movq 608(%rsp), %rdx
	leaq .Lanon.5bbf1cbe9e9f837e6488bd4d315a6b9f.44(%rip), %rcx
	jmp .LBB226_423
.LBB226_422:
	movq %r12, %rdi
	movq %rbx, %rdx
	leaq .Lanon.5bbf1cbe9e9f837e6488bd4d315a6b9f.30(%rip), %rcx
.LBB226_423:
	movb $1, %bpl
	.cfi_escape 0x2e, 0x00
	movq 16(%rsp), %rax
	movq 8(%rsp), %r13
	movq %rbx, %rsi
	movq %rax, (%rsp)
	vzeroupper
	callq *core::slice::index::slice_index_fail@GOTPCREL(%rip)
	jmp .LBB226_452
.LBB226_424:
	movb $1, %bpl
	.cfi_escape 0x2e, 0x00
	movq 16(%rsp), %rax
	movq 8(%rsp), %r13
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/index.rs:456
		slice_index_fail(self.start, self.end, slice.len())
	leaq .Lanon.5bbf1cbe9e9f837e6488bd4d315a6b9f.44(%rip), %rcx
	movq %rax, (%rsp)
	vzeroupper
	callq *core::slice::index::slice_index_fail@GOTPCREL(%rip)
	jmp .LBB226_452
.LBB226_425:
	movb $1, %bpl
	.cfi_escape 0x2e, 0x00
	movq 16(%rsp), %rax
	movq %rax, (%rsp)
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/allocator-api2-0.2.21/src/stable/raw_vec.rs:612
		Err(CapacityOverflow) => capacity_overflow(),
	callq *allocator_api2::stable::raw_vec::capacity_overflow@GOTPCREL(%rip)
	jmp .LBB226_452
.LBB226_426:
	movb $1, %bpl
	.cfi_escape 0x2e, 0x00
	movq 8(%rsp), %r13
	movq %rax, (%rsp)
	callq *allocator_api2::stable::raw_vec::capacity_overflow@GOTPCREL(%rip)
	jmp .LBB226_452
.LBB226_427:
	.cfi_escape 0x2e, 0x00
	leaq 352(%rsp), %rdi
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/raw_vec/mod.rs:675
		do_reserve_and_handle(self, len, additional, elem_layout);
	movl $1, %ecx
	movl $3, %r8d
	callq <alloc::raw_vec::RawVecInner<_>>::reserve::do_reserve_and_handle::<alloc::alloc::Global>
	jmp .LBB226_85
.LBB226_428:
	.cfi_escape 0x2e, 0x00
	leaq 96(%rsp), %rdi
	movl $4, %ecx
	movl $16, %r8d
	vzeroupper
	callq <alloc::raw_vec::RawVecInner<_>>::reserve::do_reserve_and_handle::<alloc::alloc::Global>
	jmp .LBB226_86
.LBB226_429:
	leaq .Lanon.5bbf1cbe9e9f837e6488bd4d315a6b9f.32(%rip), %rdx
	jmp .LBB226_431
.LBB226_430:
	leaq .Lanon.5bbf1cbe9e9f837e6488bd4d315a6b9f.33(%rip), %rdx
.LBB226_431:
	.cfi_escape 0x2e, 0x00
	movq 440(%rsp), %rdi
	movl $3, %esi
	callq *core::panicking::panic_bounds_check@GOTPCREL(%rip)
	jmp .LBB226_452
.LBB226_432:
	.cfi_escape 0x2e, 0x00
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/allocator-api2-0.2.21/src/stable/raw_vec.rs:272
		Err(_) => handle_alloc_error(layout),
	movl $16, %edi
	movq %rbx, %rsi
	callq *alloc::alloc::handle_alloc_error@GOTPCREL(%rip)
	jmp .LBB226_452
.LBB226_433:
	.cfi_escape 0x2e, 0x00
	movl $16, %edi
	movq %rbx, %rsi
	callq *alloc::alloc::handle_alloc_error@GOTPCREL(%rip)
	jmp .LBB226_452
.LBB226_434:
	.cfi_escape 0x2e, 0x00
	movl $8, %edi
	movq %rbx, %rsi
	callq *alloc::alloc::handle_alloc_error@GOTPCREL(%rip)
	jmp .LBB226_452
.LBB226_435:
	.cfi_escape 0x2e, 0x00
	movl $8, %edi
	movq %rbx, %rsi
	callq *alloc::alloc::handle_alloc_error@GOTPCREL(%rip)
	jmp .LBB226_452
.LBB226_436:
	movb $1, %bpl
	.cfi_escape 0x2e, 0x00
	movq 16(%rsp), %rax
	movq 32(%rsp), %rsi
	movq 8(%rsp), %r13
		// crates/impact_voxel/src/object/extraction.rs:768
		intersecting_planes.push(normalized_face_planes[plane_idx]);
	leaq .Lanon.5bbf1cbe9e9f837e6488bd4d315a6b9f.34(%rip), %rdx
	movq %rbx, %rdi
	movq %rax, (%rsp)
	vzeroupper
	callq *core::panicking::panic_bounds_check@GOTPCREL(%rip)
	jmp .LBB226_452
.LBB226_437:
	movb $1, %bpl
	.cfi_escape 0x2e, 0x00
	movq 16(%rsp), %rax
	movq 48(%rsp), %rsi
	movq 8(%rsp), %r13
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/index.rs:272
		&(*slice)[self]
	leaq .Lanon.5bbf1cbe9e9f837e6488bd4d315a6b9f.28(%rip), %rdx
	movq %rax, (%rsp)
	vzeroupper
	callq *core::panicking::panic_bounds_check@GOTPCREL(%rip)
	jmp .LBB226_452
.LBB226_438:
	movb $1, %bpl
	.cfi_escape 0x2e, 0x00
	movq 16(%rsp), %rax
	movq 8(%rsp), %r13
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/allocator-api2-0.2.21/src/stable/raw_vec.rs:613
		Err(AllocError { layout, .. }) => handle_alloc_error(layout),
	movl $4, %edi
	movq %rdx, %rsi
	movq %rax, (%rsp)
	vzeroupper
	callq *alloc::alloc::handle_alloc_error@GOTPCREL(%rip)
	jmp .LBB226_452
.LBB226_439:
	.cfi_escape 0x2e, 0x00
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/index.rs:278
		&mut (*slice)[self]
	leaq .Lanon.5bbf1cbe9e9f837e6488bd4d315a6b9f.26(%rip), %rdx
	movq %r8, %rdi
	movq %r14, %rsi
	vzeroupper
	callq *core::panicking::panic_bounds_check@GOTPCREL(%rip)
	jmp .LBB226_452
.LBB226_440:
	movq 288(%rsp), %rdi
	leaq .Lanon.5bbf1cbe9e9f837e6488bd4d315a6b9f.29(%rip), %rdx
	jmp .LBB226_417
.LBB226_441:
	.cfi_escape 0x2e, 0x00
	movq 248(%rsp), %rsi
		// crates/impact_voxel/src/object/inertia.rs:709
		let density = voxel_type_densities[voxel_type.idx()];
	leaq .Lanon.5bbf1cbe9e9f837e6488bd4d315a6b9f.642(%rip), %rdx
	vzeroupper
	callq *core::panicking::panic_bounds_check@GOTPCREL(%rip)
	jmp .LBB226_452
.LBB226_442:
	.cfi_escape 0x2e, 0x00
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/allocator-api2-0.2.21/src/stable/raw_vec.rs:612
		Err(CapacityOverflow) => capacity_overflow(),
	callq *allocator_api2::stable::raw_vec::capacity_overflow@GOTPCREL(%rip)
	jmp .LBB226_452
.LBB226_443:
	.cfi_escape 0x2e, 0x00
	callq *allocator_api2::stable::raw_vec::capacity_overflow@GOTPCREL(%rip)
	jmp .LBB226_452
.LBB226_444:
	.cfi_escape 0x2e, 0x00
	movq (%rsp), %rax
	movq 160(%rsp), %rcx
	movq 144(%rsp), %r12
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/allocator-api2-0.2.21/src/stable/raw_vec.rs:613
		Err(AllocError { layout, .. }) => handle_alloc_error(layout),
	movl $16, %edi
	movq %rbx, %rsi
	movq %rax, 136(%rsp)
	movq %rcx, 152(%rsp)
	callq *alloc::alloc::handle_alloc_error@GOTPCREL(%rip)
	jmp .LBB226_452
.LBB226_445:
	.cfi_escape 0x2e, 0x00
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/index.rs:278
		&mut (*slice)[self]
	leaq .Lanon.5bbf1cbe9e9f837e6488bd4d315a6b9f.27(%rip), %rax
	movq %rdx, %rdi
	movq %r14, %rsi
	movq %rax, %rdx
	callq *core::panicking::panic_bounds_check@GOTPCREL(%rip)
	jmp .LBB226_452
.LBB226_446:
	.cfi_escape 0x2e, 0x00
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/allocator-api2-0.2.21/src/stable/raw_vec.rs:613
		Err(AllocError { layout, .. }) => handle_alloc_error(layout),
	movl $16, %edi
	movq %rdx, %rsi
	callq *alloc::alloc::handle_alloc_error@GOTPCREL(%rip)
	jmp .LBB226_452
.LBB226_447:
	movb $1, %bpl
	.cfi_escape 0x2e, 0x00
	movq 16(%rsp), %rax
	movq 48(%rsp), %rsi
	movq 8(%rsp), %r13
	movl $8, %edi
	movq %rax, (%rsp)
	callq *alloc::alloc::handle_alloc_error@GOTPCREL(%rip)
	jmp .LBB226_452
.LBB226_448:
	movb $1, %bpl
	.cfi_escape 0x2e, 0x00
	movq 16(%rsp), %rax
	movq 8(%rsp), %r13
	movl $8, %edi
	movq %rdx, %rsi
	movq %rax, (%rsp)
	callq *alloc::alloc::handle_alloc_error@GOTPCREL(%rip)
	jmp .LBB226_452
.LBB226_449:
	.cfi_escape 0x2e, 0x00
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/allocator-api2-0.2.21/src/stable/raw_vec.rs:260
		Err(_) => capacity_overflow(),
	callq *allocator_api2::stable::raw_vec::capacity_overflow@GOTPCREL(%rip)
	jmp .LBB226_452
.LBB226_450:
	.cfi_escape 0x2e, 0x00
	vzeroupper
	callq *allocator_api2::stable::raw_vec::capacity_overflow@GOTPCREL(%rip)
	jmp .LBB226_452
.LBB226_451:
	.cfi_escape 0x2e, 0x00
	vzeroupper
	callq *allocator_api2::stable::raw_vec::capacity_overflow@GOTPCREL(%rip)
.LBB226_452:
	ud2
	movq %rax, %r15
	jmp .LBB226_522
	movq %rax, %r15
	.cfi_escape 0x2e, 0x00
	leaq 1760(%rsp), %rdi
		// crates/impact_voxel/src/object/extraction.rs:1284
		}
	callq core::ptr::drop_glue::<impact_voxel::object::split_detection::SplitDetector>
	xorl %ebp, %ebp
	xorl %eax, %eax
	xorl %r12d, %r12d
	jmp .LBB226_495
	movq 32(%rsp), %rcx
	movq %rax, %r15
	movb $1, %bpl
	movb $1, %al
	movq %r13, (%rsp)
	movq %rcx, 40(%rsp)
	jmp .LBB226_499
	movq %rax, %r15
	.cfi_escape 0x2e, 0x00
	leaq 1376(%rsp), %rdi
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mod.rs:825
		pub(crate) const unsafe fn drop_glue<T: PointeeSized>(_: &mut T)
	callq core::ptr::drop_glue::<impact_voxel::object::VoxelObject>
	jmp .LBB226_522
	movq %rax, %r15
	.cfi_escape 0x2e, 0x00
	leaq 1760(%rsp), %rdi
		// crates/impact_voxel/src/object/extraction.rs:1284
		}
	callq core::ptr::drop_glue::<impact_voxel::object::split_detection::SplitDetector>
	movq 584(%rsp), %rsi
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hashbrown-0.16.1/src/raw/mod.rs:2312
		if !self.is_empty_singleton() {
	testq %rsi, %rsi
	jne .LBB226_516
	jmp .LBB226_518
	jmp .LBB226_462
	jmp .LBB226_467
	movq 552(%rsp), %rcx
	movq %rax, %r15
	movb $1, %bpl
	movq %rcx, 144(%rsp)
	jmp .LBB226_476
.LBB226_462:
	movb $1, %r12b
	movq %rax, %r15
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/allocator-api2-0.2.21/src/stable/raw_vec.rs:333
		if mem::size_of::<T>() == 0 || self.cap == 0 {
	testq %r13, %r13
	je .LBB226_474
		// crates/impact_alloc/src/arena.rs:169
		unsafe { &*self.arena_ptr }
	movq 64(%rsp), %rax
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:2505
		Bump::<MIN_ALIGN>::dealloc(self, ptr, layout)
	movq 16(%rax), %rax
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cell.rs:555
		unsafe { *self.value.get() }
	movq 32(%rax), %rcx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/non_null.rs:1714
		self.as_ptr() == other.as_ptr()
	cmpq 24(%rsp), %rcx
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:2231
		if self.is_last_allocation(ptr) {
	jne .LBB226_474
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/alloc/layout.rs:573
		if element_size != 0 && n > Layout::max_size_for_alignment(alignment) / element_size {
	leaq (%r13,%r13,2), %rdx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mut_ptr.rs:961
		unsafe { intrinsics::offset(self, count) }
	leaq (%rcx,%rdx,8), %rcx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/mem/mod.rs:931
		crate::intrinsics::write_via_move(dest, src);
	movq %rcx, 32(%rax)
	jmp .LBB226_474
	jmp .LBB226_470
.LBB226_467:
	movq (%rsp), %rcx
	movq %rax, %r15
	movq 32(%rsp), %rax
	movb $1, %bpl
	movq %rcx, 136(%rsp)
	jmp .LBB226_523
	jmp .LBB226_491
.LBB226_470:
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/allocator-api2-0.2.21/src/stable/raw_vec.rs:333
		if mem::size_of::<T>() == 0 || self.cap == 0 {
	cmpq $0, (%rsp)
	movq %rax, %r15
	je .LBB226_473
		// crates/impact_alloc/src/arena.rs:169
		unsafe { &*self.arena_ptr }
	movq 64(%rsp), %rax
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:2505
		Bump::<MIN_ALIGN>::dealloc(self, ptr, layout)
	movq 16(%rax), %rax
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cell.rs:555
		unsafe { *self.value.get() }
	movq 32(%rax), %rcx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/non_null.rs:1714
		self.as_ptr() == other.as_ptr()
	cmpq 128(%rsp), %rcx
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:2231
		if self.is_last_allocation(ptr) {
	jne .LBB226_473
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/alloc/layout.rs:573
		if element_size != 0 && n > Layout::max_size_for_alignment(alignment) / element_size {
	imulq $56, (%rsp), %rdx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mut_ptr.rs:961
		unsafe { intrinsics::offset(self, count) }
	addq %rdx, %rcx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/mem/mod.rs:931
		crate::intrinsics::write_via_move(dest, src);
	movq %rcx, 32(%rax)
.LBB226_473:
	xorl %r12d, %r12d
.LBB226_474:
	.cfi_escape 0x2e, 0x00
	leaq 1760(%rsp), %rdi
		// crates/impact_voxel/src/object/extraction.rs:1284
		}
	callq core::ptr::drop_glue::<impact_voxel::object::split_detection::SplitDetector>
	xorl %ebp, %ebp
	xorl %eax, %eax
	jmp .LBB226_495
	movb $1, %bpl
	movq %rax, %r15
	movq %r12, 144(%rsp)
.LBB226_476:
	movq 152(%rsp), %rax
	movq %rax, 160(%rsp)
	movq 32(%rsp), %rax
	jmp .LBB226_523
	jmp .LBB226_491
	jmp .LBB226_491
	jmp .LBB226_491
	jmp .LBB226_482
.LBB226_482:
	movq %rax, %r15
	movb $1, %bpl
	jmp .LBB226_494
	movq 32(%rsp), %rcx
	movq %rax, %r15
	movb $1, %bpl
	movq %rcx, 144(%rsp)
	movq %rcx, 136(%rsp)
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/allocator-api2-0.2.21/src/stable/raw_vec.rs:333
		if mem::size_of::<T>() == 0 || self.cap == 0 {
	jmp .LBB226_527
	movq %rax, %r15
	movb $1, %bpl
	jmp .LBB226_530
	movq %rax, %r15
	movb $1, %bpl
	jmp .LBB226_507
	movq %rax, %r15
		// crates/impact_voxel/src/object/extraction.rs:1284
		}
	jmp .LBB226_510
	movb $1, %bpl
	movq %rax, %r15
	jmp .LBB226_532
	movq %rax, %r15
	movb $1, %bpl
	movb $1, %al
	movb $1, %r12b
	movq %rbx, 24(%rsp)
	movq %r14, %r13
	jmp .LBB226_492
	movq %rax, %r15
	jmp .LBB226_533
.LBB226_491:
	movq 8(%rsp), %r13
	movq %rax, %r15
	movb $1, %bpl
	movb $1, %al
	movb $1, %r12b
.LBB226_492:
	movq 16(%rsp), %rcx
	movq %rcx, (%rsp)
	jmp .LBB226_495
	movq %rax, %r15
.LBB226_494:
	movb $1, %al
	movb $1, %r12b
.LBB226_495:
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hashbrown-0.16.1/src/raw/mod.rs:3520
		.drop_inner_table::<T, _>(&self.alloc, Self::TABLE_LAYOUT);
	movq 1128(%rsp), %rcx
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hashbrown-0.16.1/src/raw/mod.rs:2312
		if !self.is_empty_singleton() {
	testq %rcx, %rcx
	je .LBB226_498
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hashbrown-0.16.1/src/raw/mod.rs:3520
		.drop_inner_table::<T, _>(&self.alloc, Self::TABLE_LAYOUT);
	movq 1152(%rsp), %rsi
	movq 1120(%rsp), %r8
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/num/uint_macros.rs:3178
		let (a, b) = intrinsics::mul_with_overflow(self as $ActualT, rhs as $ActualT);
	movq %rcx, %rdx
	shlq $4, %rdx
		// crates/impact_alloc/src/arena.rs:169
		unsafe { &*self.arena_ptr }
	movq (%rsi), %rsi
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mut_ptr.rs:1072
		unsafe { intrinsics::offset(self, intrinsics::unchecked_sub(0, count as isize)) }
	subq %rdx, %r8
	addq $-16, %r8
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:2505
		Bump::<MIN_ALIGN>::dealloc(self, ptr, layout)
	movq 16(%rsi), %rsi
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cell.rs:555
		unsafe { *self.value.get() }
	movq 32(%rsi), %rdi
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/non_null.rs:1714
		self.as_ptr() == other.as_ptr()
	cmpq %r8, %rdi
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:2231
		if self.is_last_allocation(ptr) {
	jne .LBB226_498
	leaq 33(%rdx,%rcx), %rcx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mut_ptr.rs:961
		unsafe { intrinsics::offset(self, count) }
	addq %rcx, %rdi
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/mem/mod.rs:931
		crate::intrinsics::write_via_move(dest, src);
	movq %rdi, 32(%rsi)
.LBB226_498:
		// crates/impact_voxel/src/object/extraction.rs:1284
		}
	testb %r12b, %r12b
	je .LBB226_502
.LBB226_499:
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/allocator-api2-0.2.21/src/stable/raw_vec.rs:333
		if mem::size_of::<T>() == 0 || self.cap == 0 {
	cmpq $0, (%rsp)
	je .LBB226_502
		// crates/impact_alloc/src/arena.rs:169
		unsafe { &*self.arena_ptr }
	movq 64(%rsp), %rcx
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:2505
		Bump::<MIN_ALIGN>::dealloc(self, ptr, layout)
	movq 16(%rcx), %rcx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cell.rs:555
		unsafe { *self.value.get() }
	movq 32(%rcx), %rdx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/non_null.rs:1714
		self.as_ptr() == other.as_ptr()
	cmpq 128(%rsp), %rdx
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:2231
		if self.is_last_allocation(ptr) {
	jne .LBB226_502
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/alloc/layout.rs:573
		if element_size != 0 && n > Layout::max_size_for_alignment(alignment) / element_size {
	imulq $56, (%rsp), %rsi
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mut_ptr.rs:961
		unsafe { intrinsics::offset(self, count) }
	addq %rsi, %rdx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/mem/mod.rs:931
		crate::intrinsics::write_via_move(dest, src);
	movq %rdx, 32(%rcx)
.LBB226_502:
		// crates/impact_voxel/src/object/extraction.rs:1284
		}
	testb %al, %al
	je .LBB226_506
	movq 40(%rsp), %rax
	movq %rax, 32(%rsp)
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/allocator-api2-0.2.21/src/stable/raw_vec.rs:333
		if mem::size_of::<T>() == 0 || self.cap == 0 {
	testq %r13, %r13
	jne .LBB226_507
		// crates/impact_voxel/src/object/extraction.rs:1284
		}
	testb %bpl, %bpl
	je .LBB226_505
.LBB226_510:
	movq 840(%rsp), %rsi
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/raw_vec/mod.rs:634
		if elem_layout.size() == 0 || self.cap.as_inner() == 0 {
	testq %rsi, %rsi
	jne .LBB226_537
	movq 848(%rsp), %rsi
	testq %rsi, %rsi
	jne .LBB226_538
.LBB226_512:
	movq 856(%rsp), %rsi
	testq %rsi, %rsi
	jne .LBB226_539
.LBB226_513:
	movq 864(%rsp), %rsi
	testq %rsi, %rsi
	je .LBB226_515
.LBB226_514:
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/num/uint_macros.rs:1359
		intrinsics::unchecked_mul(self, rhs)
	addq %rsi, %rsi
	.cfi_escape 0x2e, 0x00
	movq 832(%rsp), %rdi
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/alloc.rs:128
		unsafe { __rust_dealloc(ptr, layout.size(), layout.alignment()) }
	movl $2, %edx
	callq *__rustc::__rust_dealloc@GOTPCREL(%rip)
.LBB226_515:
	movq 32(%rsp), %rax
	movq %rax, 40(%rsp)
	movq 584(%rsp), %rsi
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hashbrown-0.16.1/src/raw/mod.rs:2312
		if !self.is_empty_singleton() {
	testq %rsi, %rsi
	je .LBB226_518
	jmp .LBB226_516
.LBB226_506:
	movq 40(%rsp), %rax
	movq %rax, 32(%rsp)
		// crates/impact_voxel/src/object/extraction.rs:1284
		}
	testb %bpl, %bpl
	jne .LBB226_510
.LBB226_505:
	movq 584(%rsp), %rsi
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hashbrown-0.16.1/src/raw/mod.rs:2312
		if !self.is_empty_singleton() {
	testq %rsi, %rsi
	je .LBB226_518
.LBB226_516:
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/num/uint_macros.rs:3178
		let (a, b) = intrinsics::mul_with_overflow(self as $ActualT, rhs as $ActualT);
	leaq (%rsi,%rsi,2), %rax
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/num/uint_macros.rs:896
		if intrinsics::unlikely(intrinsics::add_with_overflow(self, rhs).1) {
	leaq 39(,%rax,8), %rax
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hashbrown-0.16.1/src/raw/mod.rs:224
		size.checked_mul(buckets)?.checked_add(ctrl_align - 1)? & !(ctrl_align - 1);
	andq $-16, %rax
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hashbrown-0.16.1/src/raw/mod.rs:225
		let len = ctrl_offset.checked_add(buckets + Group::WIDTH)?;
	addq %rax, %rsi
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/allocator-api2-0.2.21/src/stable/alloc/global.rs:110
		if layout.size() != 0 {
	addq $17, %rsi
	je .LBB226_518
	movq 800(%rsp), %rdi
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mut_ptr.rs:1072
		unsafe { intrinsics::offset(self, intrinsics::unchecked_sub(0, count as isize)) }
	subq %rax, %rdi
	.cfi_escape 0x2e, 0x00
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/alloc.rs:128
		unsafe { __rust_dealloc(ptr, layout.size(), layout.alignment()) }
	movl $16, %edx
	callq *__rustc::__rust_dealloc@GOTPCREL(%rip)
.LBB226_518:
		// crates/impact_voxel/src/object/extraction.rs:1284
		}
	movq 352(%rsp), %rax
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/raw_vec/mod.rs:634
		if elem_layout.size() == 0 || self.cap.as_inner() == 0 {
	testq %rax, %rax
	je .LBB226_520
		// crates/impact_voxel/src/object/extraction.rs:1284
		}
	movq 360(%rsp), %rdi
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/num/uint_macros.rs:1359
		intrinsics::unchecked_mul(self, rhs)
	leaq (%rax,%rax,2), %rsi
	.cfi_escape 0x2e, 0x00
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/alloc.rs:128
		unsafe { __rust_dealloc(ptr, layout.size(), layout.alignment()) }
	movl $1, %edx
	callq *__rustc::__rust_dealloc@GOTPCREL(%rip)
.LBB226_520:
		// crates/impact_voxel/src/object/extraction.rs:1284
		}
	movq 96(%rsp), %rsi
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/raw_vec/mod.rs:634
		if elem_layout.size() == 0 || self.cap.as_inner() == 0 {
	testq %rsi, %rsi
	je .LBB226_522
		// crates/impact_voxel/src/object/extraction.rs:1284
		}
	movq 104(%rsp), %rdi
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/num/uint_macros.rs:1359
		intrinsics::unchecked_mul(self, rhs)
	shlq $4, %rsi
	.cfi_escape 0x2e, 0x00
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/alloc.rs:128
		unsafe { __rust_dealloc(ptr, layout.size(), layout.alignment()) }
	movl $4, %edx
	callq *__rustc::__rust_dealloc@GOTPCREL(%rip)
.LBB226_522:
	movq 40(%rsp), %rax
	xorl %ebp, %ebp
.LBB226_523:
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/allocator-api2-0.2.21/src/stable/raw_vec.rs:333
		if mem::size_of::<T>() == 0 || self.cap == 0 {
	testq %rax, %rax
	je .LBB226_526
		// crates/impact_alloc/src/arena.rs:169
		unsafe { &*self.arena_ptr }
	movq 64(%rsp), %rcx
	movq %rax, %rdx
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:2505
		Bump::<MIN_ALIGN>::dealloc(self, ptr, layout)
	movq 16(%rcx), %rax
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cell.rs:555
		unsafe { *self.value.get() }
	movq 32(%rax), %rcx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/non_null.rs:1714
		self.as_ptr() == other.as_ptr()
	cmpq 176(%rsp), %rcx
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:2231
		if self.is_last_allocation(ptr) {
	jne .LBB226_526
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/alloc/layout.rs:573
		if element_size != 0 && n > Layout::max_size_for_alignment(alignment) / element_size {
	shlq $4, %rdx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mut_ptr.rs:961
		unsafe { intrinsics::offset(self, count) }
	addq %rdx, %rcx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/mem/mod.rs:931
		crate::intrinsics::write_via_move(dest, src);
	movq %rcx, 32(%rax)
.LBB226_526:
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/allocator-api2-0.2.21/src/stable/raw_vec.rs:333
		if mem::size_of::<T>() == 0 || self.cap == 0 {
	cmpq $0, 136(%rsp)
	je .LBB226_529
.LBB226_527:
		// crates/impact_alloc/src/arena.rs:169
		unsafe { &*self.arena_ptr }
	movq 64(%rsp), %rax
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:2505
		Bump::<MIN_ALIGN>::dealloc(self, ptr, layout)
	movq 16(%rax), %rax
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cell.rs:555
		unsafe { *self.value.get() }
	movq 32(%rax), %rcx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/non_null.rs:1714
		self.as_ptr() == other.as_ptr()
	cmpq 168(%rsp), %rcx
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:2231
		if self.is_last_allocation(ptr) {
	jne .LBB226_529
	movq 136(%rsp), %rdx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/alloc/layout.rs:573
		if element_size != 0 && n > Layout::max_size_for_alignment(alignment) / element_size {
	shlq $5, %rdx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mut_ptr.rs:961
		unsafe { intrinsics::offset(self, count) }
	addq %rdx, %rcx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/mem/mod.rs:931
		crate::intrinsics::write_via_move(dest, src);
	movq %rcx, 32(%rax)
.LBB226_529:
	movq 144(%rsp), %rax
	movq %rax, 32(%rsp)
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/allocator-api2-0.2.21/src/stable/raw_vec.rs:333
		if mem::size_of::<T>() == 0 || self.cap == 0 {
	testq %rax, %rax
	je .LBB226_532
.LBB226_530:
		// crates/impact_alloc/src/arena.rs:169
		unsafe { &*self.arena_ptr }
	movq 64(%rsp), %rax
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:2505
		Bump::<MIN_ALIGN>::dealloc(self, ptr, layout)
	movq 16(%rax), %rax
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cell.rs:555
		unsafe { *self.value.get() }
	movq 32(%rax), %rcx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/non_null.rs:1714
		self.as_ptr() == other.as_ptr()
	cmpq 160(%rsp), %rcx
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:2231
		if self.is_last_allocation(ptr) {
	jne .LBB226_532
	movq 32(%rsp), %rdx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/alloc/layout.rs:573
		if element_size != 0 && n > Layout::max_size_for_alignment(alignment) / element_size {
	shlq $5, %rdx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mut_ptr.rs:961
		unsafe { intrinsics::offset(self, count) }
	addq %rdx, %rcx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/mem/mod.rs:931
		crate::intrinsics::write_via_move(dest, src);
	movq %rcx, 32(%rax)
.LBB226_532:
		// crates/impact_voxel/src/object/extraction.rs:1284
		}
	movq 72(%rsp), %rdi
	.cfi_escape 0x2e, 0x00
	callq core::ptr::drop_glue::<impact_alloc::arena::PoolArena>
.LBB226_533:
	testb %bpl, %bpl
	je .LBB226_549
	movq 384(%rsp), %rbx
	movq (%rbx), %rsi
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/raw_vec/mod.rs:634
		if elem_layout.size() == 0 || self.cap.as_inner() == 0 {
	testq %rsi, %rsi
	jne .LBB226_540
		// crates/impact_voxel/src/object/extraction.rs:1284
		}
	movq 24(%rbx), %rax
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/raw_vec/mod.rs:634
		if elem_layout.size() == 0 || self.cap.as_inner() == 0 {
	testq %rax, %rax
	jne .LBB226_541
.LBB226_536:
		// crates/impact_voxel/src/object/extraction.rs:1284
		}
	movq 152(%rbx), %rsi
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hashbrown-0.16.1/src/raw/mod.rs:2312
		if !self.is_empty_singleton() {
	testq %rsi, %rsi
	jne .LBB226_542
	jmp .LBB226_544
.LBB226_507:
		// crates/impact_alloc/src/arena.rs:169
		unsafe { &*self.arena_ptr }
	movq 64(%rsp), %rax
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:2505
		Bump::<MIN_ALIGN>::dealloc(self, ptr, layout)
	movq 16(%rax), %rax
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cell.rs:555
		unsafe { *self.value.get() }
	movq 32(%rax), %rcx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/non_null.rs:1714
		self.as_ptr() == other.as_ptr()
	cmpq 24(%rsp), %rcx
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:2231
		if self.is_last_allocation(ptr) {
	jne .LBB226_509
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/alloc/layout.rs:573
		if element_size != 0 && n > Layout::max_size_for_alignment(alignment) / element_size {
	leaq (%r13,%r13,2), %rdx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mut_ptr.rs:961
		unsafe { intrinsics::offset(self, count) }
	leaq (%rcx,%rdx,8), %rcx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/mem/mod.rs:931
		crate::intrinsics::write_via_move(dest, src);
	movq %rcx, 32(%rax)
.LBB226_509:
	movq 32(%rsp), %rax
	movq %rax, 40(%rsp)
		// crates/impact_voxel/src/object/extraction.rs:1284
		}
	testb %bpl, %bpl
	je .LBB226_505
	jmp .LBB226_510
.LBB226_537:
	.cfi_escape 0x2e, 0x00
	movq 808(%rsp), %rdi
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/alloc.rs:128
		unsafe { __rust_dealloc(ptr, layout.size(), layout.alignment()) }
	movl $1, %edx
	callq *__rustc::__rust_dealloc@GOTPCREL(%rip)
	movq 848(%rsp), %rsi
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/raw_vec/mod.rs:634
		if elem_layout.size() == 0 || self.cap.as_inner() == 0 {
	testq %rsi, %rsi
	je .LBB226_512
.LBB226_538:
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/num/uint_macros.rs:1359
		intrinsics::unchecked_mul(self, rhs)
	shlq $3, %rsi
	.cfi_escape 0x2e, 0x00
	movq 816(%rsp), %rdi
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/alloc.rs:128
		unsafe { __rust_dealloc(ptr, layout.size(), layout.alignment()) }
	movl $4, %edx
	callq *__rustc::__rust_dealloc@GOTPCREL(%rip)
	movq 856(%rsp), %rsi
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/raw_vec/mod.rs:634
		if elem_layout.size() == 0 || self.cap.as_inner() == 0 {
	testq %rsi, %rsi
	je .LBB226_513
.LBB226_539:
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/num/uint_macros.rs:1359
		intrinsics::unchecked_mul(self, rhs)
	addq %rsi, %rsi
	.cfi_escape 0x2e, 0x00
	movq 824(%rsp), %rdi
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/alloc.rs:128
		unsafe { __rust_dealloc(ptr, layout.size(), layout.alignment()) }
	movl $2, %edx
	callq *__rustc::__rust_dealloc@GOTPCREL(%rip)
	movq 864(%rsp), %rsi
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/raw_vec/mod.rs:634
		if elem_layout.size() == 0 || self.cap.as_inner() == 0 {
	testq %rsi, %rsi
	jne .LBB226_514
	jmp .LBB226_515
.LBB226_540:
		// crates/impact_voxel/src/object/extraction.rs:1284
		}
	movq 8(%rbx), %rdi
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/num/uint_macros.rs:1359
		intrinsics::unchecked_mul(self, rhs)
	shlq $4, %rsi
	.cfi_escape 0x2e, 0x00
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/alloc.rs:128
		unsafe { __rust_dealloc(ptr, layout.size(), layout.alignment()) }
	movl $4, %edx
	callq *__rustc::__rust_dealloc@GOTPCREL(%rip)
		// crates/impact_voxel/src/object/extraction.rs:1284
		}
	movq 24(%rbx), %rax
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/raw_vec/mod.rs:634
		if elem_layout.size() == 0 || self.cap.as_inner() == 0 {
	testq %rax, %rax
	je .LBB226_536
.LBB226_541:
		// crates/impact_voxel/src/object/extraction.rs:1284
		}
	movq 32(%rbx), %rdi
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/num/uint_macros.rs:1359
		intrinsics::unchecked_mul(self, rhs)
	leaq (%rax,%rax,2), %rsi
	.cfi_escape 0x2e, 0x00
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/alloc.rs:128
		unsafe { __rust_dealloc(ptr, layout.size(), layout.alignment()) }
	movl $1, %edx
	callq *__rustc::__rust_dealloc@GOTPCREL(%rip)
		// crates/impact_voxel/src/object/extraction.rs:1284
		}
	movq 152(%rbx), %rsi
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hashbrown-0.16.1/src/raw/mod.rs:2312
		if !self.is_empty_singleton() {
	testq %rsi, %rsi
	je .LBB226_544
.LBB226_542:
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/num/uint_macros.rs:3178
		let (a, b) = intrinsics::mul_with_overflow(self as $ActualT, rhs as $ActualT);
	leaq (%rsi,%rsi,2), %rax
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/num/uint_macros.rs:896
		if intrinsics::unlikely(intrinsics::add_with_overflow(self, rhs).1) {
	leaq 39(,%rax,8), %rax
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hashbrown-0.16.1/src/raw/mod.rs:224
		size.checked_mul(buckets)?.checked_add(ctrl_align - 1)? & !(ctrl_align - 1);
	andq $-16, %rax
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hashbrown-0.16.1/src/raw/mod.rs:225
		let len = ctrl_offset.checked_add(buckets + Group::WIDTH)?;
	addq %rax, %rsi
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/allocator-api2-0.2.21/src/stable/alloc/global.rs:110
		if layout.size() != 0 {
	addq $17, %rsi
	je .LBB226_544
		// crates/impact_voxel/src/object/extraction.rs:1284
		}
	movq 144(%rbx), %rdi
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mut_ptr.rs:1072
		unsafe { intrinsics::offset(self, intrinsics::unchecked_sub(0, count as isize)) }
	subq %rax, %rdi
	.cfi_escape 0x2e, 0x00
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/alloc.rs:128
		unsafe { __rust_dealloc(ptr, layout.size(), layout.alignment()) }
	movl $16, %edx
	callq *__rustc::__rust_dealloc@GOTPCREL(%rip)
.LBB226_544:
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mod.rs:825
		pub(crate) const unsafe fn drop_glue<T: PointeeSized>(_: &mut T)
	movq 48(%rbx), %rsi
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/raw_vec/mod.rs:634
		if elem_layout.size() == 0 || self.cap.as_inner() == 0 {
	testq %rsi, %rsi
	jne .LBB226_550
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mod.rs:825
		pub(crate) const unsafe fn drop_glue<T: PointeeSized>(_: &mut T)
	movq 72(%rbx), %rsi
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/raw_vec/mod.rs:634
		if elem_layout.size() == 0 || self.cap.as_inner() == 0 {
	testq %rsi, %rsi
	jne .LBB226_551
.LBB226_546:
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mod.rs:825
		pub(crate) const unsafe fn drop_glue<T: PointeeSized>(_: &mut T)
	movq 96(%rbx), %rsi
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/raw_vec/mod.rs:634
		if elem_layout.size() == 0 || self.cap.as_inner() == 0 {
	testq %rsi, %rsi
	jne .LBB226_552
.LBB226_547:
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mod.rs:825
		pub(crate) const unsafe fn drop_glue<T: PointeeSized>(_: &mut T)
	movq 120(%rbx), %rsi
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/raw_vec/mod.rs:634
		if elem_layout.size() == 0 || self.cap.as_inner() == 0 {
	testq %rsi, %rsi
	je .LBB226_549
.LBB226_548:
	movq 384(%rsp), %rax
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/num/uint_macros.rs:1359
		intrinsics::unchecked_mul(self, rhs)
	addq %rsi, %rsi
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mod.rs:825
		pub(crate) const unsafe fn drop_glue<T: PointeeSized>(_: &mut T)
	movq 128(%rax), %rdi
	.cfi_escape 0x2e, 0x00
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/alloc.rs:128
		unsafe { __rust_dealloc(ptr, layout.size(), layout.alignment()) }
	movl $2, %edx
	callq *__rustc::__rust_dealloc@GOTPCREL(%rip)
.LBB226_549:
	.cfi_escape 0x2e, 0x00
	movq %r15, %rdi
	callq _Unwind_Resume@PLT
.LBB226_550:
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mod.rs:825
		pub(crate) const unsafe fn drop_glue<T: PointeeSized>(_: &mut T)
	movq 56(%rbx), %rdi
	.cfi_escape 0x2e, 0x00
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/alloc.rs:128
		unsafe { __rust_dealloc(ptr, layout.size(), layout.alignment()) }
	movl $1, %edx
	callq *__rustc::__rust_dealloc@GOTPCREL(%rip)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mod.rs:825
		pub(crate) const unsafe fn drop_glue<T: PointeeSized>(_: &mut T)
	movq 72(%rbx), %rsi
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/raw_vec/mod.rs:634
		if elem_layout.size() == 0 || self.cap.as_inner() == 0 {
	testq %rsi, %rsi
	je .LBB226_546
.LBB226_551:
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mod.rs:825
		pub(crate) const unsafe fn drop_glue<T: PointeeSized>(_: &mut T)
	movq 80(%rbx), %rdi
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/num/uint_macros.rs:1359
		intrinsics::unchecked_mul(self, rhs)
	shlq $3, %rsi
	.cfi_escape 0x2e, 0x00
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/alloc.rs:128
		unsafe { __rust_dealloc(ptr, layout.size(), layout.alignment()) }
	movl $4, %edx
	callq *__rustc::__rust_dealloc@GOTPCREL(%rip)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mod.rs:825
		pub(crate) const unsafe fn drop_glue<T: PointeeSized>(_: &mut T)
	movq 96(%rbx), %rsi
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/raw_vec/mod.rs:634
		if elem_layout.size() == 0 || self.cap.as_inner() == 0 {
	testq %rsi, %rsi
	je .LBB226_547
.LBB226_552:
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mod.rs:825
		pub(crate) const unsafe fn drop_glue<T: PointeeSized>(_: &mut T)
	movq 104(%rbx), %rdi
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/num/uint_macros.rs:1359
		intrinsics::unchecked_mul(self, rhs)
	addq %rsi, %rsi
	.cfi_escape 0x2e, 0x00
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/alloc.rs:128
		unsafe { __rust_dealloc(ptr, layout.size(), layout.alignment()) }
	movl $2, %edx
	callq *__rustc::__rust_dealloc@GOTPCREL(%rip)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mod.rs:825
		pub(crate) const unsafe fn drop_glue<T: PointeeSized>(_: &mut T)
	movq 120(%rbx), %rsi
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/raw_vec/mod.rs:634
		if elem_layout.size() == 0 || self.cap.as_inner() == 0 {
	testq %rsi, %rsi
	jne .LBB226_548
	jmp .LBB226_549
	.cfi_escape 0x2e, 0x00
		// crates/impact_voxel/src/object/extraction.rs:639
		pub fn extract_polyhedron_with_property_transferrer(
	callq *core::panicking::panic_in_cleanup@GOTPCREL(%rip)

======================= Additional context =========================

.LCPI226_0:
	.long	0xc0228f5c

.LCPI226_1:
	.long	0x40228f5c

.LCPI226_2:
	.long	0x5f7fffff

.LCPI226_11:
	.long	0xc023d70a

.LCPI226_12:
	.long	0x3f000000

.LCPI226_13:
	.long	0x3eaaaaab

.LCPI226_14:
	.long	0x3e800000

.LCPI226_15:
	.long	0x41800000

.LCPI226_22:
	.byte	4
	.byte	0
	.byte	1
	.byte	1

.LCPI226_24:
	.byte	4
	.byte	0
	.byte	1
	.byte	3

.LCPI226_3:
	.quad	15

.LCPI226_5:
	.quad	8

.LCPI226_6:
	.quad	16

.LCPI226_7:
	.quad	24

.LCPI226_9:
	.quad	32

.LCPI226_23:
	.byte	1
	.byte	2
	.byte	4
	.byte	8
	.byte	16
	.byte	32
	.byte	64
	.byte	128

.LCPI226_25:
	.zero	8,128

.LCPI226_4:
	.quad	0
	.quad	1
	.quad	2
	.quad	3
	.quad	4
	.quad	5
	.quad	6
	.quad	7

.LCPI226_21:
	.byte	3

.LCPI226_17:
	.quad	2
	.quad	1

.LCPI226_20:
	.quad	0
	.quad	4
	.quad	1
	.quad	5

.Lexception95:
	.byte	255
	.byte	155

.Lanon.5bbf1cbe9e9f837e6488bd4d315a6b9f.16:
	.ascii	"internal error: entered unreachable code"

.Lanon.5bbf1cbe9e9f837e6488bd4d315a6b9f.26:
	.quad	.Lanon.5bbf1cbe9e9f837e6488bd4d315a6b9f.14
	.asciz	",\000\000\000\000\000\000\000\312\004\000\000.\000\000"

.Lanon.5bbf1cbe9e9f837e6488bd4d315a6b9f.27:
	.quad	.Lanon.5bbf1cbe9e9f837e6488bd4d315a6b9f.14
	.asciz	",\000\000\000\000\000\000\000\276\004\000\000.\000\000"

.Lanon.5bbf1cbe9e9f837e6488bd4d315a6b9f.28:
	.quad	.Lanon.5bbf1cbe9e9f837e6488bd4d315a6b9f.14
	.asciz	",\000\000\000\000\000\000\000\350\002\000\000,\000\000"

.Lanon.5bbf1cbe9e9f837e6488bd4d315a6b9f.29:
	.quad	.Lanon.5bbf1cbe9e9f837e6488bd4d315a6b9f.14
	.asciz	",\000\000\000\000\000\000\000Z\003\000\000M\000\000"

.Lanon.5bbf1cbe9e9f837e6488bd4d315a6b9f.30:
	.quad	.Lanon.5bbf1cbe9e9f837e6488bd4d315a6b9f.14
	.asciz	",\000\000\000\000\000\000\000a\003\000\000A\000\000"

.Lanon.5bbf1cbe9e9f837e6488bd4d315a6b9f.31:
	.quad	.Lanon.5bbf1cbe9e9f837e6488bd4d315a6b9f.14
	.asciz	",\000\000\000\000\000\000\000[\003\000\000\035\000\000"

.Lanon.5bbf1cbe9e9f837e6488bd4d315a6b9f.32:
	.quad	.Lanon.5bbf1cbe9e9f837e6488bd4d315a6b9f.14
	.asciz	",\000\000\000\000\000\000\0005\003\000\000>\000\000"

.Lanon.5bbf1cbe9e9f837e6488bd4d315a6b9f.33:
	.quad	.Lanon.5bbf1cbe9e9f837e6488bd4d315a6b9f.14
	.asciz	",\000\000\000\000\000\000\0008\003\000\000>\000\000"

.Lanon.5bbf1cbe9e9f837e6488bd4d315a6b9f.34:
	.quad	.Lanon.5bbf1cbe9e9f837e6488bd4d315a6b9f.14
	.asciz	",\000\000\000\000\000\000\000\000\003\000\0006\000\000"

.Lanon.5bbf1cbe9e9f837e6488bd4d315a6b9f.35:
	.quad	.Lanon.5bbf1cbe9e9f837e6488bd4d315a6b9f.14
	.asciz	",\000\000\000\000\000\000\000\252\002\000\000-\000\000"

.Lanon.5bbf1cbe9e9f837e6488bd4d315a6b9f.44:
	.quad	.Lanon.5bbf1cbe9e9f837e6488bd4d315a6b9f.4
	.asciz	"!\000\000\000\000\000\000\000^\f\000\000\020\000\000"

.Lanon.5bbf1cbe9e9f837e6488bd4d315a6b9f.641:
	.quad	.Lanon.5bbf1cbe9e9f837e6488bd4d315a6b9f.640
	.asciz	")\000\000\000\000\000\000\000J\002\000\000\031\000\000"

.Lanon.5bbf1cbe9e9f837e6488bd4d315a6b9f.642:
	.quad	.Lanon.5bbf1cbe9e9f837e6488bd4d315a6b9f.640
	.asciz	")\000\000\000\000\000\000\000\305\002\000\000\023\000\000"
