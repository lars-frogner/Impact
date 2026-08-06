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
	subq $2424, %rsp
	.cfi_def_cfa_offset 2480
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
	movq %rdx, 424(%rsp)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/array/mod.rs:477
		(&*old).clone()
	movq 312(%rsi), %rdx
	movq $-1, %rcx
	movq %r9, 40(%rsp)
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
	movq %r8, 656(%rsp)
	movq %rsi, 88(%rsp)
	movq %rdi, 608(%rsp)
	setne %cl
		// crates/impact_voxel/src/object.rs:3237
		let start = voxel_range.start / CHUNK_SIZE;
	shrq $4, %r10
	xorl %r13d, %r13d
	vmovdqu %ymm3, 1280(%rsp)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/num/uint_macros.rs:3758
		if r > 0 {
	addq %rax, %rcx
	movq %r10, 328(%rsp)
	movq %rcx, 336(%rsp)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/iter/range.rs:990
		if self.start < self.end {
	subq %r10, %rcx
	cmovbq %r13, %rcx
	movq %rcx, 848(%rsp)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:1916
		fn lt(&self, other: &Self) -> bool { *self <  *other }
	vmovq %xmm3, %rcx
	movq %rcx, 432(%rsp)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/num/uint_macros.rs:3758
		if r > 0 {
	vpcmpgtq %xmm0, %xmm2, %xmm0
	vpsubq %xmm0, %xmm1, %xmm0
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:1916
		fn lt(&self, other: &Self) -> bool { *self <  *other }
	vmovq %xmm0, %rax
	vmovdqu %ymm0, 1248(%rsp)
	movq %rax, 624(%rsp)
	cmpq %rax, %rcx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/iter/range.rs:900
		if self.start < self.end {
	jae .LBB226_18
	vmovdqu 1248(%rsp), %ymm0
	xorl %r13d, %r13d
	vpextrq $1, %xmm0, %rax
	vmovdqu 1280(%rsp), %ymm0
	movq %rax, 272(%rsp)
	vpextrq $1, %xmm0, %rcx
	movq %rcx, 456(%rsp)
	cmpq %rax, %rcx
	jae .LBB226_18
	movq 336(%rsp), %rax
	cmpq %rax, 328(%rsp)
	jae .LBB226_18
	movq 88(%rsp), %rax
	movq 432(%rsp), %rdx
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
	movq 328(%rsp), %rax
	movq %r8, %r11
	imulq 456(%rsp), %r11
	vpbroadcastq %rax, %zmm0
	vpaddq .LCPI226_4(%rip), %zmm0, %zmm0
	movq %rdi, %rcx
	imulq %rdx, %rcx
	movq %rax, %r10
	notq %r10
	addq 336(%rsp), %r10
	movq %rdi, 360(%rsp)
	addq %rcx, %r11
	xorl %ecx, %ecx
	leaq (%r11,%rax), %r15
	leaq 9(%r9), %rax
	movq %r15, %r14
	negq %r14
	movq %r15, 448(%rsp)
	.p2align	4
.LBB226_7:
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/num/uint_macros.rs:971
		intrinsics::unchecked_add(self, rhs)
	movq %rdi, %rbx
	imulq %rcx, %rbx
	addq 448(%rsp), %rbx
	movq 456(%rsp), %r12
	movq %rcx, 160(%rsp)
	leaq 1(%rdx), %rcx
	imulq %rdi, %rdx
	movq %r11, 248(%rsp)
	movq %r14, 592(%rsp)
	movq %r15, 368(%rsp)
	movq %rcx, 152(%rsp)
	movq %rdx, 64(%rsp)
	movq %rbx, 16(%rsp)
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
	addq 16(%rsp), %rcx
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
	movq 328(%rsp), %rdi
	jmp .LBB226_13
	.p2align	4
.LBB226_10:
	movq %r12, %rcx
	imulq %r8, %rcx
	addq 64(%rsp), %rcx
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
	addq 328(%rsp), %rdi
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
	movq 336(%rsp), %rcx
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
	jae .LBB226_408
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
	cmpq 272(%rsp), %r12
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/iter/range.rs:900
		if self.start < self.end {
	jne .LBB226_8
	movq 160(%rsp), %rcx
	movq 360(%rsp), %rdi
	movq 368(%rsp), %r15
	movq 592(%rsp), %r14
	movq 248(%rsp), %r11
	movq 152(%rsp), %rdx
	incq %rcx
	addq %rdi, %r15
	subq %rdi, %r14
	addq %rdi, %r11
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:1916
		fn lt(&self, other: &Self) -> bool { *self <  *other }
	cmpq 624(%rsp), %rdx
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
	movq 40(%rsp), %rbx
	movq %rax, 72(%rsp)
	movq %rdx, 80(%rsp)
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/allocator-api2-0.2.21/src/stable/raw_vec.rs:253
		if mem::size_of::<T>() == 0 || capacity == 0 {
	testq %rbx, %rbx
	je .LBB226_26
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/alloc/layout.rs:573
		if element_size != 0 && n > Layout::max_size_for_alignment(alignment) / element_size {
	movq %rbx, %rcx
	shrq $58, %rcx
	jne .LBB226_438
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cell.rs:555
		unsafe { *self.value.get() }
	movq 16(%rax), %rcx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/alloc/layout.rs:581
		let array_size = unsafe { unchecked_mul(element_size, n) };
	shlq $5, %rbx
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
	je .LBB226_30
	.cfi_escape 0x2e, 0x00
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:1883
		self.alloc_layout_slow(layout).ok_or(AllocErr)
	movl $16, %esi
	movq %rax, %rdi
	movq %rbx, %rdx
	callq *<bumpalo::Bump>::alloc_layout_slow@GOTPCREL(%rip)
	movq %rax, 176(%rsp)
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/allocator-api2-0.2.21/src/stable/raw_vec.rs:270
		let ptr = match result {
	testq %rax, %rax
	je .LBB226_421
		// crates/impact_alloc/src/arena.rs:169
		unsafe { &*self.arena_ptr }
	movq 72(%rsp), %rax
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/allocator-api2-0.2.21/src/stable/raw_vec.rs:270
		let ptr = match result {
	jmp .LBB226_31
.LBB226_25:
	movq 424(%rsp), %rax
		// crates/impact_voxel/src/object/extraction.rs:662
		return ExtractionResult::NotExtracted(buffers);
	vmovdqu64 (%rax), %zmm0
	vmovdqu64 112(%rax), %zmm2
	vmovdqu64 64(%rax), %zmm1
	vmovdqu64 %zmm2, 120(%rdi)
	vmovdqu64 %zmm1, 72(%rdi)
	vmovdqu64 %zmm0, 8(%rdi)
	movq $-1, (%rdi)
	jmp .LBB226_404
.LBB226_26:
	movl $16, %eax
	movq $0, 480(%rsp)
	movq $0, 136(%rsp)
	movq $0, 144(%rsp)
	movq %rax, 184(%rsp)
	movl $4, %eax
	movq %rax, 192(%rsp)
	movl $16, %eax
	movq %rax, 176(%rsp)
.LBB226_27:
	movq 424(%rsp), %rbx
	movq 88(%rsp), %r14
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/vec/mod.rs:3035
		self.len = 0;
	movq $0, 16(%rbx)
	movq $0, 40(%rbx)
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hashbrown-0.16.1/src/raw/mod.rs:873
		if self.is_empty() {
	cmpq $0, 168(%rbx)
	movq 152(%rbx), %rax
	movq %rax, 616(%rsp)
	je .LBB226_82
	movq 616(%rsp), %r15
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hashbrown-0.16.1/src/raw/mod.rs:3208
		if !self.is_empty_singleton() {
	testq %r15, %r15
	je .LBB226_80
	movq 424(%rsp), %rbx
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
	jmp .LBB226_81
.LBB226_30:
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mut_ptr.rs:470
		unsafe { intrinsics::arith_offset(self, count) as *mut T }
	subq %rbx, %rdi
	movq %rdi, 176(%rsp)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/mem/mod.rs:931
		crate::intrinsics::write_via_move(dest, src);
	movq %rdi, 32(%rcx)
.LBB226_31:
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
	je .LBB226_35
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
	je .LBB226_422
	movq %rax, 184(%rsp)
		// crates/impact_alloc/src/arena.rs:169
		unsafe { &*self.arena_ptr }
	movq 72(%rsp), %rax
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/allocator-api2-0.2.21/src/stable/raw_vec.rs:270
		let ptr = match result {
	jmp .LBB226_36
.LBB226_35:
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mut_ptr.rs:470
		unsafe { intrinsics::arith_offset(self, count) as *mut T }
	subq %rbx, %rdi
	movq %rdi, 184(%rsp)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/mem/mod.rs:931
		crate::intrinsics::write_via_move(dest, src);
	movq %rdi, 32(%rcx)
.LBB226_36:
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cell.rs:555
		unsafe { *self.value.get() }
	movq 16(%rax), %rcx
	movq 40(%rsp), %r12
	movq 656(%rsp), %rbx
	movq 32(%rcx), %rdi
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/alloc/layout.rs:581
		let array_size = unsafe { unchecked_mul(element_size, n) };
	shlq $4, %r12
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mut_ptr.rs:470
		unsafe { intrinsics::arith_offset(self, count) as *mut T }
	andq $-4, %rdi
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:1952
		if aligned_ptr < start || aligned_size > capacity {
	movq %rdi, %rdx
	subq (%rcx), %rdx
	setb %sil
	cmpq %rdx, %r12
	seta %dl
	orb %sil, %dl
	je .LBB226_40
	.cfi_escape 0x2e, 0x00
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:1883
		self.alloc_layout_slow(layout).ok_or(AllocErr)
	movl $4, %esi
	movq %rax, %rdi
	movq %r12, %rdx
	callq *<bumpalo::Bump>::alloc_layout_slow@GOTPCREL(%rip)
	movq %rax, 192(%rsp)
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/allocator-api2-0.2.21/src/stable/raw_vec.rs:270
		let ptr = match result {
	testq %rax, %rax
	jne .LBB226_41
	.cfi_escape 0x2e, 0x00
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/allocator-api2-0.2.21/src/stable/raw_vec.rs:272
		Err(_) => handle_alloc_error(layout),
	movl $4, %edi
	movq %r12, %rsi
	callq *alloc::alloc::handle_alloc_error@GOTPCREL(%rip)
	jmp .LBB226_441
.LBB226_40:
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mut_ptr.rs:470
		unsafe { intrinsics::arith_offset(self, count) as *mut T }
	subq %r12, %rdi
	movq %rdi, 192(%rsp)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/mem/mod.rs:931
		crate::intrinsics::write_via_move(dest, src);
	movq %rdi, 32(%rcx)
.LBB226_41:
	movq 40(%rsp), %rcx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mut_ptr.rs:961
		unsafe { intrinsics::offset(self, count) }
	addq %rbx, %r12
	movl $1, %edx
	xorl %r14d, %r14d
	xorl %r15d, %r15d
	movq $0, 152(%rsp)
	movq $0, 480(%rsp)
	movq %r12, 456(%rsp)
	movq %rcx, 144(%rsp)
	movq %rcx, 136(%rsp)
	jmp .LBB226_44
	.p2align	4
.LBB226_42:
	movq 184(%rsp), %r12
.LBB226_43:
	vmovdqa 16(%rsp), %xmm0
	vmovd 64(%rsp), %xmm1
	movq 656(%rsp), %rbx
	movq 368(%rsp), %rdx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/non_null.rs:1714
		self.as_ptr() == other.as_ptr()
	addq $-32, 152(%rsp)
	movq 456(%rsp), %rcx
	movq %r12, 184(%rsp)
	leaq (%rbx,%r15,8), %rax
	addq $2, %r15
	incq %rdx
	addq $16, %rax
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mod.rs:1933
		intrinsics::write_via_move(dst, src)
	vmovdqa %xmm0, (%r12,%r14)
	vmovd %xmm1, 16(%r12,%r14)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/non_null.rs:1714
		self.as_ptr() == other.as_ptr()
	addq $32, %r14
	cmpq %rcx, %rax
	je .LBB226_27
.LBB226_44:
		// crates/impact_math/src/vector.rs:1385
		self.y
	vmovsd 4(%rbx,%r15,8), %xmm0
	movq 480(%rsp), %rcx
	movq 144(%rsp), %r12
	movq 176(%rsp), %rsi
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/non_null.rs:656
		unsafe { transmute(intrinsics::offset(self.as_ptr(), count)) }
	cmpq %r15, %rdx
	movq %r15, %rbp
	movl $4, %eax
	movq %rdx, 368(%rsp)
	cmovaq %rdx, %rbp
	cmpq $5, %rbp
	cmovbq %rax, %rbp
	movq %rcx, 272(%rsp)
	vmovaps %xmm0, 16(%rsp)
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/glam-0.30.10/src/f32/sse2/vec3a.rs:109
		unsafe { UnionCast { a: [x, y, z, z] }.v }
	vmovss (%rbx,%r15,8), %xmm0
	vmovaps %xmm0, 592(%rsp)
		// crates/impact_geometry/src/plane.rs:320
		Plane::new(self.unit_normal.aligned(), self.displacement)
	vmovss 12(%rbx,%r15,8), %xmm0
	vmovss %xmm0, 64(%rsp)
		// crates/impact_voxel/src/object/extraction.rs:703
		plane.displacement() - INTERIOR_MARGIN,
	vaddss .LCPI226_11(%rip), %xmm0, %xmm0
	vmovss %xmm0, 248(%rsp)
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/allocator-api2-0.2.21/src/stable/vec/mod.rs:1793
		if self.len == self.buf.capacity() {
	cmpq %r12, %rcx
	jne .LBB226_52
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/num/uint_macros.rs:896
		if intrinsics::unlikely(intrinsics::add_with_overflow(self, rhs).1) {
	leaq 1(%r12), %rcx
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/allocator-api2-0.2.21/src/stable/raw_vec.rs:498
		let cap = cmp::max(self.cap * 2, required_cap);
	leaq (%r12,%r12), %rax
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:1038
		if other < self { self } else { other }
	movl $4, %edx
	movq %rsi, 160(%rsp)
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
	je .LBB226_53
	movq 160(%rsp), %rsi
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/alloc/layout.rs:573
		if element_size != 0 && n > Layout::max_size_for_alignment(alignment) / element_size {
	cmpq %rcx, %rax
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:966
		match self {
	ja .LBB226_431
		// crates/impact_alloc/src/arena.rs:169
		unsafe { &*self.arena_ptr }
	movq 72(%rsp), %rdi
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/alloc/layout.rs:573
		if element_size != 0 && n > Layout::max_size_for_alignment(alignment) / element_size {
	movq %r12, %r10
	shlq $5, %r10
	movq %r10, 448(%rsp)
	movq 16(%rdi), %rax
	movq 32(%rax), %rcx
	movq %rcx, %r8
	cmpq %rsi, %rcx
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:2355
		if align_is_compatible && self.is_last_allocation(ptr) {
	je .LBB226_60
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:1898
		let start = footer.data.as_ptr();
	movq (%rax), %rcx
.LBB226_49:
	movq %r8, %rbx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mut_ptr.rs:470
		unsafe { intrinsics::arith_offset(self, count) as *mut T }
	andq $-16, %rbx
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:1952
		if aligned_ptr < start || aligned_size > capacity {
	cmpq %rcx, %rbx
	jb .LBB226_57
	movq %rbx, %rsi
	subq %rcx, %rsi
	cmpq %rsi, %rdx
	ja .LBB226_57
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mut_ptr.rs:470
		unsafe { intrinsics::arith_offset(self, count) as *mut T }
	subq %rdx, %rbx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/mem/mod.rs:931
		crate::intrinsics::write_via_move(dest, src);
	movq %rbx, 32(%rax)
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:1880
		if let Some(p) = self.try_alloc_layout_fast(layout) {
	jmp .LBB226_59
	.p2align	4
.LBB226_52:
	movq %r12, 144(%rsp)
	movq %rsi, %rbx
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/allocator-api2-0.2.21/src/stable/vec/mod.rs:1793
		if self.len == self.buf.capacity() {
	jmp .LBB226_66
.LBB226_53:
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/alloc/layout.rs:573
		if element_size != 0 && n > Layout::max_size_for_alignment(alignment) / element_size {
	cmpq %rcx, %rax
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:966
		match self {
	ja .LBB226_431
		// crates/impact_alloc/src/arena.rs:169
		unsafe { &*self.arena_ptr }
	movq 72(%rsp), %rdi
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
	jb .LBB226_64
	cmpq %rcx, %rdx
	ja .LBB226_64
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mut_ptr.rs:470
		unsafe { intrinsics::arith_offset(self, count) as *mut T }
	subq %rdx, %rbx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/mem/mod.rs:931
		crate::intrinsics::write_via_move(dest, src);
	movq %rbx, 32(%rax)
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:1880
		if let Some(p) = self.try_alloc_layout_fast(layout) {
	jmp .LBB226_66
.LBB226_57:
	.cfi_escape 0x2e, 0x00
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:1883
		self.alloc_layout_slow(layout).ok_or(AllocErr)
	movl $16, %esi
	movq %r12, 584(%rsp)
	movq %rdx, 360(%rsp)
	callq *<bumpalo::Bump>::alloc_layout_slow@GOTPCREL(%rip)
	movq 360(%rsp), %rdx
	movq %rax, %rbx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/option.rs:1335
		match self {
	testq %rax, %rax
	je .LBB226_435
.LBB226_59:
	.cfi_escape 0x2e, 0x00
	movq 160(%rsp), %rsi
	movq 448(%rsp), %rdx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mod.rs:551
		unsafe { crate::intrinsics::copy_nonoverlapping(src, dst, count) }
	movq %rbx, %rdi
	callq *memcpy@GOTPCREL(%rip)
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:2371
		}
	jmp .LBB226_66
.LBB226_60:
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:2358
		let delta = new_size - old_size;
	movq %rdx, %r9
	subq %r10, %r9
	movabsq $9223372036854775793, %rcx
	cmpq %rcx, %r9
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/mem/alignment.rs:137
		if align.is_power_of_two() {
	jae .LBB226_435
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:1898
		let start = footer.data.as_ptr();
	movq (%rax), %rcx
	movq 160(%rsp), %rsi
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
	jb .LBB226_49
	cmpq %rsi, %r9
	ja .LBB226_49
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mut_ptr.rs:470
		unsafe { intrinsics::arith_offset(self, count) as *mut T }
	subq %r9, %rbx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/mem/mod.rs:931
		crate::intrinsics::write_via_move(dest, src);
	movq %rbx, 32(%rax)
	.cfi_escape 0x2e, 0x00
	movq 160(%rsp), %rsi
	movq 448(%rsp), %rdx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mod.rs:642
		crate::intrinsics::copy(src, dst, count)
	movq %rbx, %rdi
	callq *memmove@GOTPCREL(%rip)
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:2371
		}
	jmp .LBB226_66
.LBB226_64:
	.cfi_escape 0x2e, 0x00
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:1883
		self.alloc_layout_slow(layout).ok_or(AllocErr)
	movl $16, %esi
	movq $0, 584(%rsp)
	movq %rdx, 360(%rsp)
	callq *<bumpalo::Bump>::alloc_layout_slow@GOTPCREL(%rip)
	movq 360(%rsp), %rdx
	movq %rax, %rbx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:966
		match self {
	testq %rax, %rax
	je .LBB226_435
	.p2align	4
.LBB226_66:
	vpmovsxbd .LCPI226_22(%rip), %xmm0
	vmovaps 16(%rsp), %xmm1
	movq 272(%rsp), %rax
	movq %rbx, 176(%rsp)
		// crates/impact_voxel/src/object/extraction.rs:707
		plane.displacement() + EXTERIOR_MARGIN,
	leaq 1(%rax), %rcx
	movq %rcx, 480(%rsp)
	vpermt2ps 592(%rsp), %xmm0, %xmm1
	vmovss 248(%rsp), %xmm0
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mod.rs:1933
		intrinsics::write_via_move(dst, src)
	vmovaps %xmm1, (%rbx,%r14)
	vmovss %xmm0, 16(%rbx,%r14)
	vmovss 64(%rsp), %xmm0
	vmovaps %xmm1, 16(%rsp)
		// crates/impact_voxel/src/object/extraction.rs:707
		plane.displacement() + EXTERIOR_MARGIN,
	vaddss .LCPI226_1(%rip), %xmm0, %xmm0
	vmovss %xmm0, 64(%rsp)
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/allocator-api2-0.2.21/src/stable/vec/mod.rs:1793
		if self.len == self.buf.capacity() {
	cmpq 136(%rsp), %rax
	jne .LBB226_42
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:1038
		if other < self { self } else { other }
	cmpq %r15, %rcx
	movq %r15, %rax
	movl $4, %ebx
	cmovaq %rcx, %rax
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/alloc/layout.rs:573
		if element_size != 0 && n > Layout::max_size_for_alignment(alignment) / element_size {
	movabsq $288230376151711743, %rcx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:1038
		if other < self { self } else { other }
	cmpq $5, %rax
	cmovaeq %rax, %rbx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/alloc/layout.rs:573
		if element_size != 0 && n > Layout::max_size_for_alignment(alignment) / element_size {
	cmpq %rcx, %rax
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:966
		match self {
	ja .LBB226_432
		// crates/impact_alloc/src/arena.rs:169
		unsafe { &*self.arena_ptr }
	movq 72(%rsp), %rdi
	movq %rbx, 136(%rsp)
	shlq $5, %rbx
	movq 16(%rdi), %rax
	movq 32(%rax), %r12
	cmpq 184(%rsp), %r12
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:2355
		if align_is_compatible && self.is_last_allocation(ptr) {
	je .LBB226_76
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:1898
		let start = footer.data.as_ptr();
	movq (%rax), %rcx
.LBB226_70:
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mut_ptr.rs:470
		unsafe { intrinsics::arith_offset(self, count) as *mut T }
	andq $-16, %r12
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:1952
		if aligned_ptr < start || aligned_size > capacity {
	cmpq %rcx, %r12
	jb .LBB226_73
	movq %r12, %rdx
	subq %rcx, %rdx
	cmpq %rdx, %rbx
	ja .LBB226_73
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mut_ptr.rs:470
		unsafe { intrinsics::arith_offset(self, count) as *mut T }
	subq %rbx, %r12
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/mem/mod.rs:931
		crate::intrinsics::write_via_move(dest, src);
	movq %r12, 32(%rax)
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:1880
		if let Some(p) = self.try_alloc_layout_fast(layout) {
	jmp .LBB226_75
	.p2align	4
.LBB226_73:
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
	je .LBB226_433
.LBB226_75:
	.cfi_escape 0x2e, 0x00
	movq 184(%rsp), %rsi
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mod.rs:551
		unsafe { crate::intrinsics::copy_nonoverlapping(src, dst, count) }
	movq %r12, %rdi
	movq %r14, %rdx
	callq *memcpy@GOTPCREL(%rip)
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:2371
		}
	jmp .LBB226_43
.LBB226_76:
	movq 152(%rsp), %rcx
	shlq $5, %rbp
	movabsq $9223372036854775793, %rdx
	leaq (%rcx,%rbp), %rsi
	cmpq %rdx, %rsi
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/mem/alignment.rs:137
		if align.is_power_of_two() {
	jae .LBB226_433
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:1898
		let start = footer.data.as_ptr();
	movq (%rax), %rcx
	movq 184(%rsp), %r8
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
	jb .LBB226_70
	cmpq %r8, %rsi
	ja .LBB226_70
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
	movq 184(%rsp), %rsi
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mod.rs:642
		crate::intrinsics::copy(src, dst, count)
	movq %rbp, %rdi
	movq %r14, %rdx
	callq *memmove@GOTPCREL(%rip)
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:2371
		}
	jmp .LBB226_43
.LBB226_80:
	movq 424(%rsp), %rbx
	xorl %eax, %eax
.LBB226_81:
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hashbrown-0.16.1/src/raw/mod.rs:3211
		self.items = 0;
	movq $0, 168(%rbx)
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hashbrown-0.16.1/src/raw/mod.rs:3212
		self.growth_left = bucket_mask_to_capacity(self.bucket_mask);
	movq %rax, 160(%rbx)
.LBB226_82:
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
	movq %rdx, 776(%rsp)
		// crates/impact_voxel/src/object/extraction.rs:715
		invalidated_mesh_chunk_indices: poly_invalidated_mesh_chunk_indices,
	movq %rcx, 1232(%rsp)
	movq 168(%rbx), %rdx
		// crates/impact_voxel/src/object/extraction.rs:716
		split_detector_buffers: poly_split_detector_buffers,
	movq 48(%rbx), %rcx
		// crates/impact_voxel/src/object/extraction.rs:713
		chunks: mut poly_chunks,
	movq %rax, 112(%rsp)
		// crates/impact_voxel/src/object/extraction.rs:714
		voxels: mut poly_voxels,
	movq %rsi, 416(%rsp)
		// crates/impact_voxel/src/object/extraction.rs:713
		chunks: mut poly_chunks,
	vmovaps %xmm0, 96(%rsp)
		// crates/impact_voxel/src/object/extraction.rs:714
		voxels: mut poly_voxels,
	vmovdqu 24(%rbx), %xmm0
		// crates/impact_voxel/src/object/extraction.rs:715
		invalidated_mesh_chunk_indices: poly_invalidated_mesh_chunk_indices,
	movq %rdx, 1240(%rsp)
	movq %rcx, 816(%rsp)
		// crates/impact_voxel/src/object/extraction.rs:716
		split_detector_buffers: poly_split_detector_buffers,
	movq 56(%rbx), %rdx
	movq 72(%rbx), %rcx
	movq %rdx, 784(%rsp)
	movq %rcx, 824(%rsp)
	movq 80(%rbx), %rdx
	movq 96(%rbx), %rcx
		// crates/impact_voxel/src/object/extraction.rs:714
		voxels: mut poly_voxels,
	vmovdqa %xmm0, 400(%rsp)
	movq %rdx, 792(%rsp)
	movq %rcx, 832(%rsp)
		// crates/impact_voxel/src/object/extraction.rs:716
		split_detector_buffers: poly_split_detector_buffers,
	movq 104(%rbx), %rdx
	movq 120(%rbx), %rcx
	movq %rdx, 800(%rsp)
	movq %rcx, 840(%rsp)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/raw_vec/mod.rs:619
		if elem_size == 0 { usize::MAX } else { self.cap.as_inner() }
	movq 400(%rsp), %rcx
		// crates/impact_voxel/src/object/extraction.rs:716
		split_detector_buffers: poly_split_detector_buffers,
	movq 128(%rbx), %rdx
	movq %rdx, 808(%rsp)
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
	ja .LBB226_416
.LBB226_83:
	vmovdqu 1280(%rsp), %ymm1
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/vec/mod.rs:1471
		self.buf.reserve(self.len, additional);
	movq 112(%rsp), %rsi
	vpmaxuq 1248(%rsp), %xmm1, %xmm0
	vpsubq %xmm1, %xmm0, %xmm0
	vmovq %xmm0, %rax
	vpextrq $1, %xmm0, %rdx
	vmovdqa %xmm0, 1808(%rsp)
	imulq %rax, %rdx
	imulq 848(%rsp), %rdx
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
	ja .LBB226_417
.LBB226_84:
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/allocator-api2-0.2.21/src/stable/raw_vec.rs:253
		if mem::size_of::<T>() == 0 || capacity == 0 {
	testq %r13, %r13
	je .LBB226_89
	movabsq $384307168202282325, %rax
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/alloc/layout.rs:573
		if element_size != 0 && n > Layout::max_size_for_alignment(alignment) / element_size {
	cmpq %rax, %r13
	ja .LBB226_439
		// crates/impact_alloc/src/arena.rs:169
		unsafe { &*self.arena_ptr }
	movq 72(%rsp), %rdi
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
	jb .LBB226_90
	cmpq %rcx, %rbx
	ja .LBB226_90
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mut_ptr.rs:470
		unsafe { intrinsics::arith_offset(self, count) as *mut T }
	subq %rbx, %rdx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/mem/mod.rs:931
		crate::intrinsics::write_via_move(dest, src);
	movq %rdx, 32(%rax)
	movq %rdx, %rax
	jmp .LBB226_92
.LBB226_89:
	movl $8, %eax
	movq %rax, 32(%rsp)
	movq %rax, 128(%rsp)
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/allocator-api2-0.2.21/src/stable/raw_vec.rs:253
		if mem::size_of::<T>() == 0 || capacity == 0 {
	jmp .LBB226_98
.LBB226_90:
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
	je .LBB226_423
.LBB226_92:
	movabsq $164703072086692425, %rcx
	movq %rax, 32(%rsp)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/alloc/layout.rs:573
		if element_size != 0 && n > Layout::max_size_for_alignment(alignment) / element_size {
	cmpq %rcx, %r13
	ja .LBB226_440
		// crates/impact_alloc/src/arena.rs:169
		unsafe { &*self.arena_ptr }
	movq 72(%rsp), %rdi
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
	jb .LBB226_96
	cmpq %rcx, %rbx
	ja .LBB226_96
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mut_ptr.rs:470
		unsafe { intrinsics::arith_offset(self, count) as *mut T }
	subq %rbx, %rdx
	movq %rdx, 128(%rsp)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/mem/mod.rs:931
		crate::intrinsics::write_via_move(dest, src);
	movq %rdx, 32(%rax)
	jmp .LBB226_98
.LBB226_96:
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
	je .LBB226_424
.LBB226_98:
	.cfi_escape 0x2e, 0x00
	leaq 72(%rsp), %rbx
	leaq 1312(%rsp), %rdi
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hashbrown-0.16.1/src/raw/mod.rs:1699
		match Self::fallible_with_capacity(alloc, table_layout, capacity, Fallibility::Infallible) {
	movl $1, %ecx
	movq %r13, %rdx
	movq %rbx, %rsi
	vzeroupper
	callq <hashbrown::raw::RawTableInner>::fallible_with_capacity::<&impact_alloc::arena::PoolArena>
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hashbrown-0.16.1/src/map.rs:579
		Self {
	vmovdqu 1312(%rsp), %ymm0
	movq 432(%rsp), %rax
	vmovdqu %ymm0, 1040(%rsp)
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hashbrown-0.16.1/src/set.rs:596
		Self {
	movq %rbx, 1072(%rsp)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:1916
		fn lt(&self, other: &Self) -> bool { *self <  *other }
	cmpq 624(%rsp), %rax
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/iter/range.rs:900
		if self.start < self.end {
	jae .LBB226_353
	vmovdqu 1248(%rsp), %ymm0
	vpextrq $1, %xmm0, %rax
	vmovdqu 1280(%rsp), %ymm0
	movq %rax, 1112(%rsp)
	vpextrq $1, %xmm0, %rdx
	cmpq %rax, %rdx
	jae .LBB226_353
	movq 336(%rsp), %rax
	cmpq %rax, 328(%rsp)
	jae .LBB226_353
	movq 2480(%rsp), %rax
	movq 480(%rsp), %rsi
	movq 176(%rsp), %rcx
	xorl %ebp, %ebp
	xorl %ebx, %ebx
	movq %r13, 16(%rsp)
	movq $0, 296(%rsp)
	movq %rdx, 1096(%rsp)
	vmovss 40(%rax), %xmm0
	vbroadcastss 32(%rax), %xmm1
	shlq $5, %rsi
	addq %rsi, %rcx
	movq %rsi, 1160(%rsp)
	movq %rcx, 1168(%rsp)
	leaq 24(%r14), %rcx
	movq %rcx, 1088(%rsp)
	leaq 48(%r14), %rcx
	movq %rcx, 632(%rsp)
	leaq 152(%r14), %rcx
	movq %rcx, 312(%rsp)
	movq 24(%rax), %rcx
	vmovss %xmm0, 456(%rsp)
	vmovss 36(%rax), %xmm0
	vmulss .LCPI226_12(%rip), %xmm0, %xmm2
	vmulss .LCPI226_13(%rip), %xmm0, %xmm0
	vmovaps %xmm1, 272(%rsp)
	movq %rcx, 248(%rsp)
	movq 16(%rax), %rcx
	movq %rcx, 592(%rsp)
	movq (%rax), %rcx
	movq 8(%rax), %rax
	vmovss %xmm0, 360(%rsp)
	vmulss .LCPI226_14(%rip), %xmm1, %xmm0
	vmulss .LCPI226_15(%rip), %xmm1, %xmm1
	vmovss %xmm2, 448(%rsp)
	movq %rax, 160(%rsp)
	movabsq $576460752303423488, %rax
	movq %rcx, 368(%rsp)
	xorl %ecx, %ecx
	decq %rax
	movq %rax, 864(%rsp)
	movq 40(%rsp), %rax
	movq %rax, 48(%rsp)
	movl %eax, 376(%rsp)
	vmovss %xmm0, 584(%rsp)
	vmulss %xmm1, %xmm1, %xmm0
	vmovaps %xmm1, 1872(%rsp)
	vmulss %xmm0, %xmm1, %xmm2
	vmovss %xmm2, 716(%rsp)
	vmulss .LCPI226_12(%rip), %xmm0, %xmm2
	vmulss .LCPI226_13(%rip), %xmm0, %xmm0
	vmovss %xmm2, 712(%rsp)
	vmulss .LCPI226_14(%rip), %xmm1, %xmm2
	vbroadcastss %xmm1, %xmm1
	vmovss %xmm0, 708(%rsp)
	vmovaps %xmm1, 1856(%rsp)
	vmovss %xmm2, 704(%rsp)
.LBB226_103:
	movq 432(%rsp), %rax
	movq %rcx, 304(%rsp)
	movq %rbx, 560(%rsp)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/num/uint_macros.rs:971
		intrinsics::unchecked_add(self, rhs)
	leaq 1(%rax), %rcx
	shlq $4, %rax
	vcvtusi2ss %rax, %xmm15, %xmm0
	addq $16, %rax
	movq %rcx, 1104(%rsp)
	vmovaps %xmm0, 1840(%rsp)
	vcvtusi2ss %rax, %xmm15, %xmm0
	vmovaps %xmm0, 1824(%rsp)
	jmp .LBB226_106
	.p2align	4
.LBB226_104:
	movq 8(%rsp), %rax
	movq (%rsp), %r13
	movq 88(%rsp), %r14
	movq %rax, 16(%rsp)
.LBB226_105:
	movq 1120(%rsp), %rdx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:1916
		fn lt(&self, other: &Self) -> bool { *self <  *other }
	cmpq 1112(%rsp), %rdx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/iter/range.rs:900
		if self.start < self.end {
	je .LBB226_352
.LBB226_106:
	movq %rdx, %rcx
	shlq $4, %rcx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/num/uint_macros.rs:971
		intrinsics::unchecked_add(self, rhs)
	leaq 1(%rdx), %rax
	movq %rdx, 872(%rsp)
	vcvtusi2ss %rcx, %xmm15, %xmm1
	vmovaps 1840(%rsp), %xmm0
	addq $16, %rcx
	movq %rax, 1120(%rsp)
	vmovaps %xmm1, 1936(%rsp)
	vinsertps $16, %xmm1, %xmm0, %xmm0
	vcvtusi2ss %rcx, %xmm15, %xmm1
	vmovaps %xmm0, 1904(%rsp)
	vmovaps 1824(%rsp), %xmm0
	movq 328(%rsp), %rcx
	vmovaps %xmm1, 1920(%rsp)
	vinsertps $16, %xmm1, %xmm0, %xmm0
	vmovaps %xmm0, 1888(%rsp)
	jmp .LBB226_108
	.p2align	4
.LBB226_107:
	movq 880(%rsp), %rcx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:1916
		fn lt(&self, other: &Self) -> bool { *self <  *other }
	cmpq 336(%rsp), %rcx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/iter/range.rs:900
		if self.start < self.end {
	jae .LBB226_105
.LBB226_108:
	movq 208(%r14), %rsi
	movq 216(%r14), %rbx
	movq 16(%rsp), %rax
	movq %r14, %rdx
	movq 8(%r14), %r14
	movq %r13, (%rsp)
	movq %rbp, 576(%rsp)
	movq %rsi, 232(%rsp)
	movq %rbx, 264(%rsp)
	imulq 432(%rsp), %rsi
	imulq 872(%rsp), %rbx
	movq %rax, 8(%rsp)
	movq %rcx, %rax
	addq %rsi, %rbx
	movq 16(%rdx), %rsi
	movq %rsi, 152(%rsp)
	jmp .LBB226_110
	.p2align	4
.LBB226_109:
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/raw_vec/mod.rs:614
		self.ptr.cast().as_non_null_ptr()
	movq 104(%rsp), %rax
	movq 880(%rsp), %rdx
	movq 152(%rsp), %rsi
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mut_ptr.rs:961
		unsafe { intrinsics::offset(self, count) }
	movq %r15, %rcx
	shlq $4, %rcx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/vec/mod.rs:1046
		self.len = len + 1;
	incq %r15
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mod.rs:1933
		intrinsics::write_via_move(dst, src)
	movb $3, 9(%rax,%rcx)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/vec/mod.rs:1046
		self.len = len + 1;
	movq %r15, 112(%rsp)
	movq %rdx, %rax
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:1916
		fn lt(&self, other: &Self) -> bool { *self <  *other }
	cmpq 336(%rsp), %rdx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/iter/range.rs:900
		if self.start < self.end {
	jae .LBB226_104
.LBB226_110:
	movq 432(%rsp), %rcx
	movq 872(%rsp), %rdx
	leaq (%rbx,%rax), %rdi
		// crates/impact_voxel/src/object/extraction.rs:741
		let chunk_indices = [chunk_i, chunk_j, chunk_k];
	movq %rcx, 208(%rsp)
	movq %rdx, 216(%rsp)
	movq %rax, 224(%rsp)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/index.rs:272
		&(*slice)[self]
	cmpq %rsi, %rdi
	jae .LBB226_426
	leaq 1(%rax), %rcx
	movq %rdi, 320(%rsp)
	shlq $4, %rdi
		// crates/impact_voxel/src/object/extraction.rs:746
		if let VoxelChunk::Void = chunk {
	movl $2, %edx
	movq %rcx, 880(%rsp)
		// crates/impact_voxel/src/object/extraction.rs:744
		let chunk = self.chunks[chunk_idx];
	movl (%r14,%rdi), %ecx
	movzbl 9(%r14,%rdi), %r8d
	movl 4(%r14,%rdi), %r12d
	movzbl 8(%r14,%rdi), %r9d
	movq %rcx, 64(%rsp)
	movzwl 14(%r14,%rdi), %ecx
		// crates/impact_voxel/src/object/extraction.rs:746
		if let VoxelChunk::Void = chunk {
	movl %r8d, %esi
	subb $3, %sil
	movzbl %sil, %r10d
	cmovbl %edx, %r10d
		// crates/impact_voxel/src/object/extraction.rs:744
		let chunk = self.chunks[chunk_idx];
	movw %cx, 468(%rsp)
	movl 10(%r14,%rdi), %ecx
	movl %ecx, 464(%rsp)
		// crates/impact_voxel/src/object/extraction.rs:746
		if let VoxelChunk::Void = chunk {
	testb %r10b, %r10b
	je .LBB226_117
		// crates/impact_voxel/src/object.rs:873
		(chunk_k * CHUNK_SIZE) as f32,
	shlq $4, %rax
	vmovaps 1904(%rsp), %xmm2
	vmovaps 1888(%rsp), %xmm3
	movq 184(%rsp), %rcx
	movq %rdi, 344(%rsp)
	addq %r14, %rdi
	movq %r12, 352(%rsp)
	vcvtusi2ss %rax, %xmm15, %xmm0
		// crates/impact_voxel/src/object.rs:878
		((chunk_k + 1) * CHUNK_SIZE) as f32,
	addq $16, %rax
	vcvtusi2ss %rax, %xmm15, %xmm1
	movq 1160(%rsp), %rax
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/glam-0.30.10/src/f32/sse2/vec3a.rs:109
		unsafe { UnionCast { a: [x, y, z, z] }.v }
	vshufps $4, %xmm0, %xmm2, %xmm2
	vshufps $4, %xmm1, %xmm3, %xmm3
		// crates/impact_geometry/src/axis_aligned_box.rs:42
		Self {
	vmovaps %xmm2, 2032(%rsp)
	vmovaps %xmm3, 2048(%rsp)
	.p2align	4
.LBB226_113:
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/non_null.rs:1714
		self.as_ptr() == other.as_ptr()
	testq %rax, %rax
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/iter/macros.rs:180
		if ptr == crate::intrinsics::transmute::<$ptr, NonNull<T>>(end_or_len) {
	je .LBB226_119
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/iter/macros.rs:332
		if f(x) {
	vmovaps (%rcx), %xmm2
	vmovaps 1936(%rsp), %xmm4
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/../../stdarch/crates/core_arch/src/x86/sse.rs:1130
		simd_bitmask::<i32x4, u8>(mask) as i32
	vmovmskps %xmm2, %edx
		// crates/impact_geometry/src/axis_aligned_box.rs:648
		(x & 0b010) | ((x & 0b001) << 2) | ((x & 0b100) >> 2)
	vmovd %edx, %xmm3
	vgf2p8affineqb $0, .LCPI226_23(%rip){1to2}, %xmm3, %xmm3
	vmovd %xmm3, %edx
	vmovaps 1920(%rsp), %xmm3
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
	vmovss 2032(%rsp,%rdx,4), %xmm5
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
	jbe .LBB226_113
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/vec/mod.rs:1037
		let len = self.len;
	movq 112(%rsp), %r15
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/vec/mod.rs:1040
		if len == self.buf.capacity() {
	cmpq 96(%rsp), %r15
	jne .LBB226_109
	.cfi_escape 0x2e, 0x00
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/vec/mod.rs:1041
		self.buf.grow_one();
	leaq 96(%rsp), %rdi
	vzeroupper
	callq *<alloc::raw_vec::RawVec<impact_voxel::voxel_types::FixedVoxelMaterialProperties>>::grow_one@GOTPCREL(%rip)
	jmp .LBB226_109
	.p2align	4
.LBB226_117:
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/vec/mod.rs:1037
		let len = self.len;
	movq 112(%rsp), %r15
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/vec/mod.rs:1040
		if len == self.buf.capacity() {
	cmpq 96(%rsp), %r15
	jne .LBB226_109
	.cfi_escape 0x2e, 0x00
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/vec/mod.rs:1041
		self.buf.grow_one();
	leaq 96(%rsp), %rdi
	vzeroupper
	callq *<alloc::raw_vec::RawVec<impact_voxel::voxel_types::FixedVoxelMaterialProperties>>::grow_one@GOTPCREL(%rip)
	jmp .LBB226_109
	.p2align	4
.LBB226_119:
	cmpq $0, 480(%rsp)
	leaq 9(%rdi), %rax
	movb %r8b, 172(%rsp)
	movb %r9b, 204(%rsp)
	movq %rax, 240(%rsp)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/iter/macros.rs:180
		if ptr == crate::intrinsics::transmute::<$ptr, NonNull<T>>(end_or_len) {
	je .LBB226_227
	movq 656(%rsp), %r15
	movq 176(%rsp), %r14
	xorl %ebx, %ebx
	movl %r10d, 260(%rsp)
	movq %rdi, 440(%rsp)
	movq $0, 520(%rsp)
	jmp .LBB226_124
	.p2align	4
.LBB226_121:
	movq 192(%rsp), %r13
	movq %rax, 16(%rsp)
.LBB226_122:
	movq 520(%rsp), %rcx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mod.rs:1933
		intrinsics::write_via_move(dst, src)
	vmovups (%r15), %xmm0
	movq %r13, 192(%rsp)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mut_ptr.rs:961
		unsafe { intrinsics::offset(self, count) }
	movq %rcx, %rax
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/allocator-api2-0.2.21/src/stable/vec/mod.rs:1799
		self.len += 1;
	incq %rcx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mut_ptr.rs:961
		unsafe { intrinsics::offset(self, count) }
	shlq $4, %rax
	movq %rcx, 520(%rsp)
	movq 16(%rsp), %rcx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mod.rs:1933
		intrinsics::write_via_move(dst, src)
	vmovups %xmm0, (%r13,%rax)
	movq %rcx, 48(%rsp)
.LBB226_123:
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/non_null.rs:1714
		self.as_ptr() == other.as_ptr()
	addq $16, %r15
	addq $32, %r14
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/iter/adapters/enumerate.rs:82
		self.count += 1;
	incq %rbx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/non_null.rs:1714
		self.as_ptr() == other.as_ptr()
	cmpq 1168(%rsp), %r14
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/iter/macros.rs:180
		if ptr == crate::intrinsics::transmute::<$ptr, NonNull<T>>(end_or_len) {
	je .LBB226_148
.LBB226_124:
		// crates/impact_voxel/src/object/extraction.rs:766
		for (plane_idx, inner_plane) in inner_planes.iter().enumerate() {
	testq %r14, %r14
	je .LBB226_148
		// crates/impact_voxel/src/object/extraction.rs:767
		if !chunk_aabb.lies_in_negative_halfspace_of_plane(inner_plane) {
	vmovaps (%r14), %xmm0
		// crates/impact_math/src/point.rs:312
		self.inner.y
	vmovsd 2052(%rsp), %xmm3
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/glam-0.30.10/src/f32/sse2/vec3a.rs:109
		unsafe { UnionCast { a: [x, y, z, z] }.v }
	vpmovsxbd .LCPI226_24(%rip), %xmm4
		// crates/impact_voxel/src/object/extraction.rs:767
		if !chunk_aabb.lies_in_negative_halfspace_of_plane(inner_plane) {
	vmovss 16(%r14), %xmm1
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
	vmovsd 2036(%rsp), %xmm2
		// crates/impact_geometry/src/axis_aligned_box.rs:168
		if is_lower_x {
	andl $4, %eax
		// crates/impact_math/src/point.rs:312
		self.inner.y
	vmovaps %xmm2, %xmm3 {%k1}
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/glam-0.30.10/src/f32/sse2/vec3a.rs:109
		unsafe { UnionCast { a: [x, y, z, z] }.v }
	vmovss 2032(%rsp,%rax,4), %xmm2
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
	ja .LBB226_123
		// crates/impact_voxel/src/object/extraction.rs:768
		intersecting_planes.push(normalized_face_planes[plane_idx]);
	cmpq 40(%rsp), %rbx
	jae .LBB226_425
	movq 48(%rsp), %rax
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/allocator-api2-0.2.21/src/stable/vec/mod.rs:1793
		if self.len == self.buf.capacity() {
	cmpq %rax, 520(%rsp)
	jne .LBB226_121
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/num/uint_macros.rs:896
		if intrinsics::unlikely(intrinsics::add_with_overflow(self, rhs).1) {
	cmpq $-1, %rax
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/intrinsics/mod.rs:459
		if b {
	je .LBB226_407
	movq 48(%rsp), %rsi
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:1038
		if other < self { self } else { other }
	movl $4, %r12d
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
	cmovaeq %rax, %r12
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/alloc/layout.rs:573
		if element_size != 0 && n > Layout::max_size_for_alignment(alignment) / element_size {
	movq %r12, %rdx
	shlq $4, %rdx
	movq %r12, 16(%rsp)
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/allocator-api2-0.2.21/src/stable/raw_vec.rs:333
		if mem::size_of::<T>() == 0 || self.cap == 0 {
	testq %rsi, %rsi
	je .LBB226_136
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/alloc/layout.rs:573
		if element_size != 0 && n > Layout::max_size_for_alignment(alignment) / element_size {
	cmpq 864(%rsp), %rax
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:966
		match self {
	ja .LBB226_407
		// crates/impact_alloc/src/arena.rs:169
		unsafe { &*self.arena_ptr }
	movq 72(%rsp), %rdi
	movq 48(%rsp), %rcx
	movq 16(%rdi), %rax
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/alloc/layout.rs:573
		if element_size != 0 && n > Layout::max_size_for_alignment(alignment) / element_size {
	shlq $4, %rcx
	movq %rcx, 56(%rsp)
	movq 32(%rax), %r13
	cmpq 192(%rsp), %r13
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:2355
		if align_is_compatible && self.is_last_allocation(ptr) {
	je .LBB226_143
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:1898
		let start = footer.data.as_ptr();
	movq (%rax), %rcx
.LBB226_133:
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mut_ptr.rs:470
		unsafe { intrinsics::arith_offset(self, count) as *mut T }
	andq $-4, %r13
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:1952
		if aligned_ptr < start || aligned_size > capacity {
	cmpq %rcx, %r13
	jb .LBB226_140
	movq %r13, %rsi
	subq %rcx, %rsi
	cmpq %rsi, %rdx
	ja .LBB226_140
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mut_ptr.rs:470
		unsafe { intrinsics::arith_offset(self, count) as *mut T }
	subq %rdx, %r13
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/mem/mod.rs:931
		crate::intrinsics::write_via_move(dest, src);
	movq %r13, 32(%rax)
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:1880
		if let Some(p) = self.try_alloc_layout_fast(layout) {
	jmp .LBB226_142
.LBB226_136:
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/alloc/layout.rs:573
		if element_size != 0 && n > Layout::max_size_for_alignment(alignment) / element_size {
	cmpq 864(%rsp), %rax
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:966
		match self {
	ja .LBB226_407
		// crates/impact_alloc/src/arena.rs:169
		unsafe { &*self.arena_ptr }
	movq 72(%rsp), %rdi
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
	je .LBB226_147
	.cfi_escape 0x2e, 0x00
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:1883
		self.alloc_layout_slow(layout).ok_or(AllocErr)
	movl $4, %esi
	movq %rdx, 56(%rsp)
	vzeroupper
	callq *<bumpalo::Bump>::alloc_layout_slow@GOTPCREL(%rip)
	movq 56(%rsp), %rdx
	movq %rax, %r13
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:966
		match self {
	testq %rax, %rax
	jne .LBB226_122
	jmp .LBB226_427
.LBB226_140:
	.cfi_escape 0x2e, 0x00
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:1883
		self.alloc_layout_slow(layout).ok_or(AllocErr)
	movl $4, %esi
	movq %rdx, %r12
	vzeroupper
	callq *<bumpalo::Bump>::alloc_layout_slow@GOTPCREL(%rip)
	movq %rax, %r13
	movq %r12, %rdx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/option.rs:1335
		match self {
	testq %rax, %rax
	je .LBB226_427
.LBB226_142:
	.cfi_escape 0x2e, 0x00
	movq 192(%rsp), %rsi
	movq 56(%rsp), %rdx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mod.rs:551
		unsafe { crate::intrinsics::copy_nonoverlapping(src, dst, count) }
	movq %r13, %rdi
	vzeroupper
	callq *memcpy@GOTPCREL(%rip)
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:2371
		}
	jmp .LBB226_122
.LBB226_143:
	movabsq $9223372036854775793, %rsi
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:2358
		let delta = new_size - old_size;
	movq %rdx, %r8
	subq %rcx, %r8
	leaq 12(%rsi), %rcx
	cmpq %rcx, %r8
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/mem/alignment.rs:137
		if align.is_power_of_two() {
	jae .LBB226_427
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:1898
		let start = footer.data.as_ptr();
	movq (%rax), %rcx
	movq 192(%rsp), %rsi
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:453
		ptr.wrapping_sub(ptr as usize & (divisor - 1))
	andl $3, %esi
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mut_ptr.rs:470
		unsafe { intrinsics::arith_offset(self, count) as *mut T }
	movq %r13, %r12
	subq %rsi, %r12
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:1952
		if aligned_ptr < start || aligned_size > capacity {
	movq %r12, %rsi
	subq %rcx, %rsi
	jb .LBB226_133
	cmpq %rsi, %r8
	ja .LBB226_133
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mut_ptr.rs:470
		unsafe { intrinsics::arith_offset(self, count) as *mut T }
	subq %r8, %r12
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/mem/mod.rs:931
		crate::intrinsics::write_via_move(dest, src);
	movq %r12, 32(%rax)
	.cfi_escape 0x2e, 0x00
	movq 192(%rsp), %rsi
	movq 56(%rsp), %rdx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mod.rs:642
		crate::intrinsics::copy(src, dst, count)
	movq %r12, %rdi
	vzeroupper
	callq *memmove@GOTPCREL(%rip)
	movq %r12, %r13
	jmp .LBB226_122
.LBB226_147:
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mut_ptr.rs:470
		unsafe { intrinsics::arith_offset(self, count) as *mut T }
	addq $-64, %r13
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/mem/mod.rs:931
		crate::intrinsics::write_via_move(dest, src);
	movq %r13, 32(%rax)
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:1880
		if let Some(p) = self.try_alloc_layout_fast(layout) {
	jmp .LBB226_122
	.p2align	4
.LBB226_148:
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/allocator-api2-0.2.21/src/stable/vec/mod.rs:2043
		self.len() == 0
	cmpq $0, 520(%rsp)
	movq 88(%rsp), %rax
	movl 260(%rsp), %r10d
		// crates/impact_voxel/src/object/extraction.rs:778
		if is_fully_inside {
	je .LBB226_228
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/vec/mod.rs:3054
		let len = self.len;
	movq 416(%rsp), %rcx
	movq 344(%rsp), %r12
	movq 152(%rsp), %rsi
	movq %rcx, 648(%rsp)
	movq 440(%rsp), %rcx
	leaq 9(%rcx), %rdi
		// crates/impact_voxel/src/object.rs:2535
		if let &mut Self::Uniform(UniformVoxelChunk {
	cmpb $4, (%rdi)
	jne .LBB226_155
	movq %rax, %rdx
		// crates/impact_voxel/src/object.rs:2537
		split_detection,
	movl (%rcx), %eax
		// crates/impact_voxel/src/object.rs:2536
		voxel,
	movzwl 4(%rcx), %r14d
	movzbl 6(%rcx), %r15d
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/raw_vec/mod.rs:619
		if elem_size == 0 { usize::MAX } else { self.cap.as_inner() }
	movq 24(%rdx), %rcx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/vec/mod.rs:3054
		let len = self.len;
	movq 40(%rdx), %rbx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/num/uint_macros.rs:2621
		intrinsics::wrapping_sub(self, rhs)
	subq %rbx, %rcx
	movl %eax, 64(%rsp)
	movq %rbx, %rax
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/raw_vec/mod.rs:767
		additional > self.capacity(elem_layout.size()).wrapping_sub(len)
	cmpq $4095, %rcx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/raw_vec/mod.rs:673
		if self.needs_to_grow(len, additional, elem_layout) {
	jbe .LBB226_350
.LBB226_151:
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mut_ptr.rs:961
		unsafe { intrinsics::offset(self, count) }
	leaq (%rax,%rax,2), %rcx
	addq 32(%rdx), %rcx
	shll $16, %r15d
	xorl %edx, %edx
	orl %r15d, %r14d
	.p2align	4
.LBB226_152:
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mod.rs:1933
		intrinsics::write_via_move(dst, src)
	movl %r14d, %esi
	shrl $16, %esi
	movw %r14w, (%rcx,%rdx)
	movb %sil, 2(%rcx,%rdx)
	movw %r14w, 3(%rcx,%rdx)
	movb %sil, 5(%rcx,%rdx)
	movw %r14w, 6(%rcx,%rdx)
	movb %sil, 8(%rcx,%rdx)
	movw %r14w, 9(%rcx,%rdx)
	movb %sil, 11(%rcx,%rdx)
	movw %r14w, 12(%rcx,%rdx)
	movb %sil, 14(%rcx,%rdx)
	movw %r14w, 15(%rcx,%rdx)
	movb %sil, 17(%rcx,%rdx)
	movw %r14w, 18(%rcx,%rdx)
	movb %sil, 20(%rcx,%rdx)
	movw %r14w, 21(%rcx,%rdx)
	movb %sil, 23(%rcx,%rdx)
	movw %r14w, 24(%rcx,%rdx)
	movb %sil, 26(%rcx,%rdx)
	movw %r14w, 27(%rcx,%rdx)
	movb %sil, 29(%rcx,%rdx)
	movw %r14w, 30(%rcx,%rdx)
	movb %sil, 32(%rcx,%rdx)
	movw %r14w, 33(%rcx,%rdx)
	movb %sil, 35(%rcx,%rdx)
	movw %r14w, 36(%rcx,%rdx)
	movb %sil, 38(%rcx,%rdx)
	movw %r14w, 39(%rcx,%rdx)
	movb %sil, 41(%rcx,%rdx)
	movw %r14w, 42(%rcx,%rdx)
	movb %sil, 44(%rcx,%rdx)
	movw %r14w, 45(%rcx,%rdx)
	movb %sil, 47(%rcx,%rdx)
	movw %r14w, 48(%rcx,%rdx)
	movb %sil, 50(%rcx,%rdx)
	movw %r14w, 51(%rcx,%rdx)
	movb %sil, 53(%rcx,%rdx)
	movw %r14w, 54(%rcx,%rdx)
	movb %sil, 56(%rcx,%rdx)
	movw %r14w, 57(%rcx,%rdx)
	movb %sil, 59(%rcx,%rdx)
	movw %r14w, 60(%rcx,%rdx)
	movb %sil, 62(%rcx,%rdx)
	movw %r14w, 63(%rcx,%rdx)
	movb %sil, 65(%rcx,%rdx)
	movw %r14w, 66(%rcx,%rdx)
	movb %sil, 68(%rcx,%rdx)
	movw %r14w, 69(%rcx,%rdx)
	movb %sil, 71(%rcx,%rdx)
	movw %r14w, 72(%rcx,%rdx)
	movb %sil, 74(%rcx,%rdx)
	movw %r14w, 75(%rcx,%rdx)
	movb %sil, 77(%rcx,%rdx)
	movw %r14w, 78(%rcx,%rdx)
	movb %sil, 80(%rcx,%rdx)
	movw %r14w, 81(%rcx,%rdx)
	movb %sil, 83(%rcx,%rdx)
	movw %r14w, 84(%rcx,%rdx)
	movb %sil, 86(%rcx,%rdx)
	movw %r14w, 87(%rcx,%rdx)
	movb %sil, 89(%rcx,%rdx)
	movw %r14w, 90(%rcx,%rdx)
	movb %sil, 92(%rcx,%rdx)
	movw %r14w, 93(%rcx,%rdx)
	movb %sil, 95(%rcx,%rdx)
	movw %r14w, 96(%rcx,%rdx)
	movb %sil, 98(%rcx,%rdx)
	movw %r14w, 99(%rcx,%rdx)
	movb %sil, 101(%rcx,%rdx)
	movw %r14w, 102(%rcx,%rdx)
	movb %sil, 104(%rcx,%rdx)
	movw %r14w, 105(%rcx,%rdx)
	movb %sil, 107(%rcx,%rdx)
	movw %r14w, 108(%rcx,%rdx)
	movb %sil, 110(%rcx,%rdx)
	movw %r14w, 111(%rcx,%rdx)
	movb %sil, 113(%rcx,%rdx)
	movw %r14w, 114(%rcx,%rdx)
	movb %sil, 116(%rcx,%rdx)
	movw %r14w, 117(%rcx,%rdx)
	movb %sil, 119(%rcx,%rdx)
	movw %r14w, 120(%rcx,%rdx)
	movb %sil, 122(%rcx,%rdx)
	movw %r14w, 123(%rcx,%rdx)
	movb %sil, 125(%rcx,%rdx)
	movw %r14w, 126(%rcx,%rdx)
	movb %sil, 128(%rcx,%rdx)
	movw %r14w, 129(%rcx,%rdx)
	movb %sil, 131(%rcx,%rdx)
	movw %r14w, 132(%rcx,%rdx)
	movb %sil, 134(%rcx,%rdx)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:1916
		fn lt(&self, other: &Self) -> bool { *self <  *other }
	addq $135, %rdx
	cmpq $12285, %rdx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/iter/range.rs:900
		if self.start < self.end {
	jne .LBB226_152
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mod.rs:1933
		intrinsics::write_via_move(dst, src)
	movw %r14w, (%rcx,%rdx)
	movb %sil, 2(%rcx,%rdx)
	movq 88(%rsp), %rdx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/vec/set_len_on_drop.rs:19
		self.local_len += increment;
	addq $4096, %rax
		// crates/impact_voxel/src/object.rs:3154
		(start_voxel_idx >> (3 * LOG2_CHUNK_SIZE)) as u32
	shrq $12, %rbx
	movq %rax, 40(%rdx)
	movq 440(%rsp), %rax
		// crates/impact_voxel/src/object.rs:2542
		*self = Self::NonUniform(NonUniformVoxelChunk {
	movl %ebx, (%rax)
	movl $65537, 4(%rax)
	movb $63, 8(%rax)
	movw $257, 4(%rdi)
	movl $16843009, (%rdi)
	.cfi_escape 0x2e, 0x00
	movq 8(%rsp), %rax
	movq 632(%rsp), %rdi
	movl 64(%rsp), %esi
	movq (%rsp), %r13
	movq %rax, 16(%rsp)
		// crates/impact_voxel/src/object.rs:2548
		split_detector.convert_uniform_chunk_to_non_uniform(split_detection);
	vzeroupper
	callq *<impact_voxel::object::split_detection::SplitDetector>::convert_uniform_chunk_to_non_uniform@GOTPCREL(%rip)
	movq 88(%rsp), %rax
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/vec/mod.rs:1903
		&mut *core::intrinsics::aggregate_raw_ptr::<*mut [T], _, _>(self.as_mut_ptr(), self.len)
	movq 16(%rax), %rsi
.LBB226_155:
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/index.rs:278
		&mut (*slice)[self]
	cmpq %rsi, 320(%rsp)
	jae .LBB226_429
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/raw_vec/mod.rs:614
		self.ptr.cast().as_non_null_ptr()
	movq 8(%rax), %rdx
		// crates/impact_voxel/src/object/extraction.rs:858
		let VoxelChunk::NonUniform(chunk) = &mut self.chunks[chunk_idx] else {
	cmpb $2, 9(%rdx,%r12)
	ja .LBB226_409
	addq %r12, %rdx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/vec/mod.rs:1903
		&mut *core::intrinsics::aggregate_raw_ptr::<*mut [T], _, _>(self.as_mut_ptr(), self.len)
	movq 40(%rax), %rcx
		// crates/impact_voxel/src/object/extraction.rs:862
		let chunk_voxels = chunk_voxels_mut(&mut self.voxels, chunk.data_offset);
	movl (%rdx), %r14d
	movq %rdx, 856(%rsp)
	movq %rcx, 640(%rsp)
		// crates/impact_voxel/src/object.rs:3149
		(data_offset as usize) << (3 * LOG2_CHUNK_SIZE)
	movq %r14, %r15
	shlq $12, %r15
		// crates/impact_voxel/src/object.rs:3166
		&mut voxels[start_voxel_idx..start_voxel_idx + CHUNK_VOXEL_COUNT]
	leaq 4096(%r15), %rbx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/index.rs:451
		&& self.end <= slice.len()
	cmpq %rcx, %rbx
	ja .LBB226_410
		// crates/impact_voxel/src/object/extraction.rs:862
		let chunk_voxels = chunk_voxels_mut(&mut self.voxels, chunk.data_offset);
	movq 32(%rax), %rcx
	movq 648(%rsp), %r12
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/raw_vec/mod.rs:619
		if elem_size == 0 { usize::MAX } else { self.cap.as_inner() }
	movq 400(%rsp), %rax
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/num/uint_macros.rs:2621
		intrinsics::wrapping_sub(self, rhs)
	subq %r12, %rax
	movq %r12, %rbx
	movq %rcx, 568(%rsp)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/raw_vec/mod.rs:767
		additional > self.capacity(elem_layout.size()).wrapping_sub(len)
	cmpq $4095, %rax
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/raw_vec/mod.rs:673
		if self.needs_to_grow(len, additional, elem_layout) {
	jbe .LBB226_346
.LBB226_159:
	movq %r14, %r13
	leaq (%r15,%r15,2), %r14
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/raw_vec/mod.rs:614
		self.ptr.cast().as_non_null_ptr()
	movq 408(%rsp), %r15
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mut_ptr.rs:961
		unsafe { intrinsics::offset(self, count) }
	leaq (%rbx,%rbx,2), %rax
	addq %rcx, %r14
	movq %rax, 1152(%rsp)
	leaq (%r15,%rax), %rdi
	.cfi_escape 0x2e, 0x00
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mod.rs:551
		unsafe { crate::intrinsics::copy_nonoverlapping(src, dst, count) }
	movl $12288, %edx
	movq %r14, %rsi
	vzeroupper
	callq *memcpy@GOTPCREL(%rip)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/vec/mod.rs:2939
		self.len += count;
	addq $4096, %rbx
	movq %rbx, 416(%rsp)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/index.rs:580
		if self.start > slice.len() {
	cmpq %rbx, %r12
	ja .LBB226_411
		// crates/impact_math/src/point.rs:306
		self.inner.x
	vmovsd 2032(%rsp), %xmm1
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/array/mod.rs:598
		self.try_map(NeverShortCircuit::wrap_mut_1(f)).0
	movq 216(%rsp), %rsi
		// crates/impact_math/src/point.rs:660
		Point3C::new(a.x + b.x(), a.y + b.y(), a.z + b.z())
	vmovss .LCPI226_12(%rip), %xmm0
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/index.rs:101
		let ptr = unsafe { crate::intrinsics::offset(ptr, offset) };
	leaq (%r12,%r12,2), %rcx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/array/mod.rs:598
		self.try_map(NeverShortCircuit::wrap_mut_1(f)).0
	movq 208(%rsp), %rdx
	movq 224(%rsp), %rdi
	movl $16, 172(%rsp)
	movq $0, 680(%rsp)
	movq %r14, 1144(%rsp)
	movq $0, 688(%rsp)
	movq $0, 664(%rsp)
	movq $0, 672(%rsp)
	movq $0, 904(%rsp)
	movq $0, 912(%rsp)
	movq $0, 888(%rsp)
	movq $0, 896(%rsp)
	movq $0, 920(%rsp)
	movq $0, 928(%rsp)
	movl $16, 380(%rsp)
	movl $16, 388(%rsp)
	movq $0, 696(%rsp)
	movl $16, 396(%rsp)
	movl $16, 392(%rsp)
	movl $16, 384(%rsp)
	movq $0, 496(%rsp)
	movq $0, 504(%rsp)
	movq $0, 512(%rsp)
	movl $16, 204(%rsp)
	movl $0, 260(%rsp)
	movl $16, 440(%rsp)
	movl $0, 556(%rsp)
	movq $0, 984(%rsp)
	movq $0, 976(%rsp)
	movq $0, 56(%rsp)
	movl $16, 552(%rsp)
	movl $0, 548(%rsp)
	movl $0, 544(%rsp)
	xorl %ebx, %ebx
	xorl %r12d, %r12d
	movl $16, 540(%rsp)
	movl $0, 536(%rsp)
	movl $16, 532(%rsp)
	movl $0, 528(%rsp)
	movq $0, 968(%rsp)
	movq $0, 960(%rsp)
	movq $0, 952(%rsp)
	movq $0, 944(%rsp)
	movq $0, 936(%rsp)
		// crates/impact_math/src/point.rs:660
		Point3C::new(a.x + b.x(), a.y + b.y(), a.z + b.z())
	vaddps .LCPI226_12(%rip){1to4}, %xmm1, %xmm1
	vaddss 2040(%rsp), %xmm0, %xmm0
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/index.rs:101
		let ptr = unsafe { crate::intrinsics::offset(ptr, offset) };
	leaq (%r15,%rcx), %rax
	movq %rcx, 1128(%rsp)
	movq %rax, 1136(%rsp)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/iter/range.rs:900
		if self.start < self.end {
	leaq 2(%r15,%rcx), %rax
		// crates/impact_voxel/src/object/extraction.rs:867
		let chunk_start_voxel_indices = chunk_indices.map(|idx| idx * CHUNK_SIZE);
	shlq $4, %rsi
	movq %rax, 1216(%rsp)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/iter/range.rs:900
		if self.start < self.end {
	leaq (%r13,%r13,2), %rax
		// crates/impact_voxel/src/object/extraction.rs:867
		let chunk_start_voxel_indices = chunk_indices.map(|idx| idx * CHUNK_SIZE);
	shlq $4, %rdx
	shlq $4, %rdi
	xorl %r13d, %r13d
	movq %rsi, 1224(%rsp)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/iter/range.rs:900
		if self.start < self.end {
	leaq -1(%r15,%rcx), %rsi
	movq 568(%rsp), %rcx
	movq %rdx, 1176(%rsp)
	movq %rdi, 152(%rsp)
	xorl %edx, %edx
	xorl %r15d, %r15d
	shlq $12, %rax
		// crates/impact_math/src/point.rs:499
		Self { x, y, z }
	vmovlps %xmm1, 992(%rsp)
		// crates/impact_voxel/src/object/extraction.rs:876
		[VoxelSignedDistance::maximally_inside(); CHUNK_SIZE];
	vmovddup .LCPI226_25(%rip), %xmm1
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/iter/range.rs:900
		if self.start < self.end {
	leaq 2(%rcx,%rax), %rax
		// crates/impact_math/src/point.rs:499
		Self { x, y, z }
	vmovss %xmm0, 1000(%rsp)
	movq %rax, 1208(%rsp)
		// crates/impact_voxel/src/object/extraction.rs:876
		[VoxelSignedDistance::maximally_inside(); CHUNK_SIZE];
	vmovaps %xmm1, 1696(%rsp)
	jmp .LBB226_162
	.p2align	4
.LBB226_161:
	movq 1200(%rsp), %rsi
	movq 1192(%rsp), %r14
	movq 1184(%rsp), %rax
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/iter/range.rs:900
		if self.start < self.end {
	addq %rcx, %r8
	movq %r8, %rdx
	addq $768, %rsi
	addq $768, %r14
	movq %rax, %r13
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:1916
		fn lt(&self, other: &Self) -> bool { *self <  *other }
	cmpq $16, %rax
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/iter/range.rs:900
		if self.start < self.end {
	je .LBB226_204
.LBB226_162:
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/num/uint_macros.rs:971
		intrinsics::unchecked_add(self, rhs)
	leaq 1(%r13), %rax
	movq %r14, 1192(%rsp)
	movq %r14, 64(%rsp)
	movq %rsi, 1200(%rsp)
	movq %rsi, %r14
	xorl %r11d, %r11d
	movq %r13, 16(%rsp)
	movq %rax, 1184(%rsp)
	movq 1176(%rsp), %rax
		// crates/impact_voxel/src/object/extraction.rs:896
		let obj_i = chunk_start_voxel_indices[0] + i as usize;
	addq %r13, %rax
	vcvtusi2ss %rax, %xmm15, %xmm0
	vmulss 272(%rsp), %xmm0, %xmm0
	vmovaps %xmm0, 1952(%rsp)
	jmp .LBB226_165
	.p2align	4
.LBB226_163:
	movq 944(%rsp), %r11
	movq 936(%rsp), %r9
		// crates/impact_voxel/src/object.rs:2942
		self.0[1][0] += count;
	addq %r15, %r11
	addq %r8, %r9
	movq %r11, 944(%rsp)
	movq %r11, 688(%rsp)
	movq %r9, 936(%rsp)
	movq %r9, 912(%rsp)
.LBB226_164:
	movq 232(%rsp), %rbx
	movq 352(%rsp), %r15
	movq 264(%rsp), %rax
	movq 344(%rsp), %r8
	movq 240(%rsp), %r11
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/iter/range.rs:900
		if self.start < self.end {
	addq $48, 64(%rsp)
	addq %rdx, %r12
	addq $48, %r14
	movq %r12, 56(%rsp)
	addq %rsi, %rax
	addq %rdi, %r15
	leaq (%r8,%rcx), %rdx
	addq %r10, %rbx
	movq %rax, %r12
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:1916
		fn lt(&self, other: &Self) -> bool { *self <  *other }
	cmpq $16, %r11
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/iter/range.rs:900
		if self.start < self.end {
	je .LBB226_161
.LBB226_165:
	movq %r12, 264(%rsp)
	movq %rbx, 232(%rsp)
	movq %rdx, %r12
	movq %r11, %rbx
	.cfi_escape 0x2e, 0x00
	movq 192(%rsp), %rsi
	movq 520(%rsp), %rdx
		// crates/impact_voxel/src/object/extraction.rs:907
		Self::compute_max_plane_signed_dists_for_row(
	leaq 1696(%rsp), %rdi
	leaq 992(%rsp), %rcx
	movl %r13d, %r8d
	movl %ebx, %r9d
	callq <impact_voxel::object::VoxelObject>::compute_max_plane_signed_dists_for_row
	leaq 1(%rbx), %rax
	vmovaps 1952(%rsp), %xmm1
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/iter/range.rs:900
		if self.start < self.end {
	leaq (%r12,%r12,2), %rdi
	movq %r15, 352(%rsp)
	movq %r12, 344(%rsp)
	xorl %r9d, %r9d
	xorl %ecx, %ecx
	xorl %edx, %edx
	xorl %esi, %esi
	movq %rax, 240(%rsp)
	movq 1224(%rsp), %rax
		// crates/impact_voxel/src/object/extraction.rs:902
		let obj_j = chunk_start_voxel_indices[1] + j as usize;
	addq %rbx, %rax
	vcvtusi2ss %rax, %xmm15, %xmm0
	vmulss 272(%rsp), %xmm0, %xmm0
	movq 1216(%rsp), %rax
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/iter/range.rs:900
		if self.start < self.end {
	leaq (%rax,%rdi), %r8
	addq 1208(%rsp), %rdi
	vinsertps $16, %xmm0, %xmm1, %xmm0
	jmp .LBB226_169
	.p2align	4
.LBB226_167:
		// crates/impact_voxel/src/object/extraction.rs:984
		row_mask |= ((voxel_signed_distance.encoded as u8 as u16)
	shrb $7, %bpl
	movzbl %bpl, %ebp
	shlxl %ecx, %ebp, %ebp
	orl %ebp, %esi
.LBB226_168:
		// crates/impact_voxel/src/object/extraction.rs:994
		*NonUniformVoxelChunk::get_voxel_mut(
	movb %r10b, -2(%r8,%r9)
	movb %r11b, -1(%r8,%r9)
	movb %al, (%r8,%r9)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:1916
		fn lt(&self, other: &Self) -> bool { *self <  *other }
	addq $3, %r9
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/num/uint_macros.rs:971
		intrinsics::unchecked_add(self, rhs)
	incq %rcx
	movq %r13, %rbp
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:1916
		fn lt(&self, other: &Self) -> bool { *self <  *other }
	cmpq $48, %r9
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/iter/range.rs:900
		if self.start < self.end {
	je .LBB226_192
.LBB226_169:
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:1038
		if other < self { self } else { other }
	movzbl 1696(%rsp,%rcx), %eax
	movq %rbp, %r13
	movzbl -1(%rdi,%r9), %r11d
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/num/int_macros.rs:1960
		intrinsics::saturating_add(self, rhs)
	movl $127, %r15d
		// crates/impact_voxel/src/object/extraction.rs:925
		let mut poly_voxel = *voxel;
	movzbl (%rdi,%r9), %r12d
	movzbl -2(%rdi,%r9), %r10d
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/num/int_macros.rs:1960
		intrinsics::saturating_add(self, rhs)
	movl %eax, %ebp
	incb %bpl
	movzbl %bpl, %ebp
	cmovol %r15d, %ebp
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/num/int_macros.rs:2046
		intrinsics::saturating_sub(0, self)
	negb %bpl
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:1038
		if other < self { self } else { other }
	movzbl %bpl, %ebp
	cmpb %r11b, %bpl
	cmovlel %r11d, %ebp
	cmpb %r11b, %al
	cmovgl %eax, %r11d
		// crates/impact_voxel/src/lib.rs:75
		bitflags! {
	movl %r12d, %eax
		// crates/impact_voxel/src/object/extraction.rs:945
		voxel.signed_distance = voxel_signed_distance;
	movb %bpl, -1(%rdi,%r9)
		// crates/impact_voxel/src/lib.rs:75
		bitflags! {
	orb $1, %al
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/num/int_macros.rs:3750
		pub const fn is_negative(self) -> bool { self < 0 }
	testb %r11b, %r11b
		// crates/impact_voxel/src/object/extraction.rs:949
		if poly_voxel_signed_distance.is_negative() {
	js .LBB226_176
	cmpq $0, 16(%rsp)
		// crates/impact_voxel/src/object/extraction.rs:1860
		if i > 0 {
	je .LBB226_172
		// crates/impact_voxel/src/lib.rs:75
		bitflags! {
	andb $-33, -765(%r14,%r9)
.LBB226_172:
	testq %rbx, %rbx
		// crates/impact_voxel/src/object/extraction.rs:1863
		if j > 0 {
	je .LBB226_174
		// crates/impact_voxel/src/lib.rs:75
		bitflags! {
	andb $-65, -45(%r14,%r9)
.LBB226_174:
		// crates/impact_voxel/src/object/extraction.rs:1866
		if k > 0 {
	testq %r9, %r9
	je .LBB226_167
		// crates/impact_voxel/src/lib.rs:75
		bitflags! {
	andb $127, (%r14,%r9)
	jmp .LBB226_167
	.p2align	4
.LBB226_176:
	movzbl %r10b, %ebp
	movb %al, (%rdi,%r9)
		// crates/impact_voxel/src/object/inertia.rs:586
		let voxel_density = voxel_type_densities[voxel_type.idx()];
	cmpq %rbp, 248(%rsp)
	jbe .LBB226_405
	movq 152(%rsp), %rax
	vmovaps 272(%rsp), %xmm3
	movq 592(%rsp), %r15
	addq %rcx, %rax
	vmovss (%r15,%rbp,4), %xmm1
	movq 160(%rsp), %r15
		// crates/impact_voxel/src/object/inertia.rs:588
		let lower_coords = Vector3::from(object_voxel_indices.map(|index| voxel_extent * index as f32));
	vcvtusi2ss %rax, %xmm15, %xmm2
		// crates/impact_voxel/src/object/inertia.rs:604
		let moments_of_inertia = ((1.0 / 3.0) * voxel_extent_pow_2 * voxel_density)
	vmulss 360(%rsp), %xmm1, %xmm6
	movq 368(%rsp), %rax
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
	vmulss 448(%rsp), %xmm1, %xmm5
		// crates/impact_voxel/src/object/inertia.rs:600
		let mass = voxel_extent_pow_3 * voxel_density;
	vmulss 456(%rsp), %xmm1, %xmm4
		// crates/impact_voxel/src/object/inertia.rs:607
		let products_of_inertia = (0.25 * voxel_extent * voxel_density)
	vmulss 584(%rsp), %xmm1, %xmm1
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
	vmovss 48(%rax), %xmm3
	vsubss %xmm4, %xmm3, %xmm3
	vmovss %xmm3, 48(%rax)
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/glam-0.30.10/src/f32/sse2/vec3a.rs:1733
		self.0 = unsafe { _mm_sub_ps(self.0, rhs.0) };
	vmovaps (%rax), %xmm3
	vmovaps 16(%rax), %xmm6
	vmovaps 32(%rax), %xmm7
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/../../stdarch/crates/core_arch/src/x86/sse.rs:62
		unsafe { simd_sub(a, b) }
	vsubps %xmm5, %xmm3, %xmm3
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/glam-0.30.10/src/f32/sse2/vec3a.rs:1733
		self.0 = unsafe { _mm_sub_ps(self.0, rhs.0) };
	vmovaps %xmm3, (%rax)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/../../stdarch/crates/core_arch/src/x86/sse.rs:62
		unsafe { simd_sub(a, b) }
	vsubps %xmm2, %xmm6, %xmm3
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/glam-0.30.10/src/f32/sse2/vec3a.rs:1733
		self.0 = unsafe { _mm_sub_ps(self.0, rhs.0) };
	vmovaps %xmm3, 16(%rax)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/../../stdarch/crates/core_arch/src/x86/sse.rs:62
		unsafe { simd_sub(a, b) }
	vsubps %xmm1, %xmm7, %xmm3
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/glam-0.30.10/src/f32/sse2/vec3a.rs:1733
		self.0 = unsafe { _mm_sub_ps(self.0, rhs.0) };
	vmovaps %xmm3, 32(%rax)
	movq 16(%rsp), %rax
		// crates/impact_voxel/src/object/inertia.rs:418
		self.destination.mass += voxel_mass;
	vaddss 48(%r15), %xmm4, %xmm3
	vmovss %xmm3, 48(%r15)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/../../stdarch/crates/core_arch/src/x86/sse.rs:36
		unsafe { simd_add(a, b) }
	vaddps (%r15), %xmm5, %xmm3
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/glam-0.30.10/src/f32/sse2/vec3a.rs:1609
		self.0 = unsafe { _mm_add_ps(self.0, rhs.0) };
	vmovaps %xmm3, (%r15)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/../../stdarch/crates/core_arch/src/x86/sse.rs:36
		unsafe { simd_add(a, b) }
	vaddps 16(%r15), %xmm2, %xmm2
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/glam-0.30.10/src/f32/sse2/vec3a.rs:1609
		self.0 = unsafe { _mm_add_ps(self.0, rhs.0) };
	vmovaps %xmm2, 16(%r15)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/../../stdarch/crates/core_arch/src/x86/sse.rs:36
		unsafe { simd_add(a, b) }
	vaddps 32(%r15), %xmm1, %xmm1
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/glam-0.30.10/src/f32/sse2/vec3a.rs:1609
		self.0 = unsafe { _mm_add_ps(self.0, rhs.0) };
	vmovaps %xmm1, 32(%r15)
	movq 64(%rsp), %r15
	testq %rax, %rax
		// crates/impact_voxel/src/object/extraction.rs:1880
		if i > 0 {
	je .LBB226_179
		// crates/impact_voxel/src/lib.rs:75
		bitflags! {
	andb $-33, -766(%r15,%r9)
	cmpq $15, %rax
		// crates/impact_voxel/src/object/extraction.rs:1883
		if i + 1 < CHUNK_SIZE {
	je .LBB226_180
.LBB226_179:
		// crates/impact_voxel/src/lib.rs:75
		bitflags! {
	andb $-5, 770(%r15,%r9)
.LBB226_180:
	testq %rbx, %rbx
		// crates/impact_voxel/src/object/extraction.rs:1886
		if j > 0 {
	je .LBB226_182
		// crates/impact_voxel/src/lib.rs:75
		bitflags! {
	andb $-65, -46(%r15,%r9)
	cmpq $15, %rbx
		// crates/impact_voxel/src/object/extraction.rs:1889
		if j + 1 < CHUNK_SIZE {
	je .LBB226_183
.LBB226_182:
		// crates/impact_voxel/src/lib.rs:75
		bitflags! {
	andb $-9, 50(%r15,%r9)
.LBB226_183:
		// crates/impact_voxel/src/object/extraction.rs:1892
		if k > 0 {
	testq %r9, %r9
	je .LBB226_185
		// crates/impact_voxel/src/lib.rs:75
		bitflags! {
	andb $127, -1(%r15,%r9)
		// crates/impact_voxel/src/object/extraction.rs:1895
		if k + 1 < CHUNK_SIZE {
	cmpq $45, %r9
	je .LBB226_186
.LBB226_185:
		// crates/impact_voxel/src/lib.rs:75
		bitflags! {
	andb $-17, 5(%r15,%r9)
.LBB226_186:
	testq %rax, %rax
		// crates/impact_voxel/src/object/extraction.rs:1828
		if i > 0 {
	je .LBB226_191
		// crates/impact_voxel/src/object/extraction.rs:1817
		let is_negative_mask = adjacent_voxel.signed_distance.is_negative_mask();
	movzbl -766(%r14,%r9), %ebp
		// crates/impact_voxel/src/lib.rs:75
		bitflags! {
	andb $-6, %r12b
		// crates/impact_voxel/src/lib.rs:234
		(self.encoded >> 7) as u8
	sarb $7, %bpl
		// crates/impact_voxel/src/lib.rs:346
		Self::from_bits_retain(self.bits() & mask)
	movl %ebp, %eax
	andb $32, %bpl
		// crates/impact_voxel/src/lib.rs:75
		bitflags! {
	orb %bpl, -765(%r14,%r9)
		// crates/impact_voxel/src/lib.rs:346
		Self::from_bits_retain(self.bits() & mask)
	andb $4, %al
		// crates/impact_voxel/src/lib.rs:75
		bitflags! {
	orb %r12b, %al
	testq %rbx, %rbx
		// crates/impact_voxel/src/object/extraction.rs:1835
		if j > 0 {
	je .LBB226_189
.LBB226_188:
		// crates/impact_voxel/src/object/extraction.rs:1817
		let is_negative_mask = adjacent_voxel.signed_distance.is_negative_mask();
	movzbl -46(%r14,%r9), %ebp
		// crates/impact_voxel/src/lib.rs:75
		bitflags! {
	andb $-9, %al
		// crates/impact_voxel/src/lib.rs:234
		(self.encoded >> 7) as u8
	sarb $7, %bpl
		// crates/impact_voxel/src/lib.rs:346
		Self::from_bits_retain(self.bits() & mask)
	movl %ebp, %r12d
	andb $64, %bpl
		// crates/impact_voxel/src/lib.rs:75
		bitflags! {
	orb %bpl, -45(%r14,%r9)
		// crates/impact_voxel/src/lib.rs:346
		Self::from_bits_retain(self.bits() & mask)
	andb $8, %r12b
		// crates/impact_voxel/src/lib.rs:75
		bitflags! {
	orb %r12b, %al
.LBB226_189:
	btsl %ecx, %edx
		// crates/impact_voxel/src/object/extraction.rs:1892
		if k > 0 {
	testq %r9, %r9
		// crates/impact_voxel/src/object/extraction.rs:1842
		if k > 0 {
	je .LBB226_168
		// crates/impact_voxel/src/object/extraction.rs:1817
		let is_negative_mask = adjacent_voxel.signed_distance.is_negative_mask();
	movzbl -1(%r14,%r9), %ebp
		// crates/impact_voxel/src/lib.rs:75
		bitflags! {
	andb $-17, %al
		// crates/impact_voxel/src/lib.rs:234
		(self.encoded >> 7) as u8
	sarb $7, %bpl
		// crates/impact_voxel/src/lib.rs:346
		Self::from_bits_retain(self.bits() & mask)
	movl %ebp, %r12d
	andb $-128, %bpl
		// crates/impact_voxel/src/lib.rs:75
		bitflags! {
	orb %bpl, (%r14,%r9)
		// crates/impact_voxel/src/lib.rs:346
		Self::from_bits_retain(self.bits() & mask)
	andb $16, %r12b
		// crates/impact_voxel/src/lib.rs:75
		bitflags! {
	orb %r12b, %al
	jmp .LBB226_168
.LBB226_191:
	andb $-2, %r12b
	movl %r12d, %eax
	testq %rbx, %rbx
		// crates/impact_voxel/src/object/extraction.rs:1835
		if j > 0 {
	jne .LBB226_188
	jmp .LBB226_189
	.p2align	4
.LBB226_192:
		// crates/impact_voxel/src/object/extraction.rs:1009
		if row_mask != 0 {
	testw %si, %si
	je .LBB226_194
	movl 204(%rsp), %r9d
	movq 16(%rsp), %r8
	movl 260(%rsp), %r10d
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/num/uint_macros.rs:154
		return intrinsics::ctlz(self as $ActualT);
	lzcntw %si, %ax
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/num/uint_macros.rs:178
		return intrinsics::cttz(self);
	tzcntl %esi, %edi
	movl 552(%rsp), %r11d
	movl 548(%rsp), %r15d
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/num/uint_macros.rs:2621
		intrinsics::wrapping_sub(self, rhs)
	xorl $31, %eax
	movzwl %ax, %eax
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:1077
		if other < self { other } else { self }
	cmpl %r9d, %r8d
	cmovbl %r8d, %r9d
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:1038
		if other < self { self } else { other }
	cmpl %r10d, %r8d
	cmoval %r8d, %r10d
	movl 172(%rsp), %r8d
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:1077
		if other < self { other } else { self }
	cmpl %r11d, %ebx
	movl %r9d, 204(%rsp)
	movl %r9d, 388(%rsp)
	cmovbl %ebx, %r11d
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:1038
		if other < self { self } else { other }
	cmpl %r15d, %ebx
	movl %r10d, 260(%rsp)
	cmoval %ebx, %r15d
	movl %r11d, 552(%rsp)
	movl %r11d, 380(%rsp)
	movl %r15d, 548(%rsp)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:1077
		if other < self { other } else { self }
	cmpl %r8d, %edi
	cmovbl %edi, %r8d
	movl %r8d, 172(%rsp)
	movl 544(%rsp), %r8d
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:1038
		if other < self { self } else { other }
	cmpl %r8d, %eax
	cmoval %eax, %r8d
	movl %r15d, %eax
	movq %rax, 920(%rsp)
	movl %r8d, %eax
	movq %rax, 928(%rsp)
	movl %r10d, %eax
	movl %r8d, 544(%rsp)
	movq %rax, 696(%rsp)
.LBB226_194:
	movq 56(%rsp), %rax
	movzwl %si, %edi
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/num/uint_macros.rs:85
		return intrinsics::ctpop(self);
	movzwl %dx, %esi
	popcntl %edi, %r15d
		// crates/impact_voxel/src/object/extraction.rs:1024
		let poly_row_occupied_count = poly_row_mask.count_ones() as usize;
	popcntl %esi, %r8d
	movq %rax, 56(%rsp)
		// crates/impact_voxel/src/object/extraction.rs:1028
		if poly_row_mask != 0 {
	testw %si, %si
	je .LBB226_196
	movq %r8, %rbp
	movl 440(%rsp), %r10d
	movq 16(%rsp), %r8
	movl 556(%rsp), %r11d
	movl 540(%rsp), %r12d
	movq %rsi, %r9
	movq %rdi, %rsi
	movq %r15, %rdi
	movl 536(%rsp), %r15d
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/num/uint_macros.rs:154
		return intrinsics::ctlz(self as $ActualT);
	lzcntw %dx, %ax
	movl 532(%rsp), %r13d
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/num/uint_macros.rs:178
		return intrinsics::cttz(self);
	tzcntl %edx, %edx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/num/uint_macros.rs:2621
		intrinsics::wrapping_sub(self, rhs)
	xorl $31, %eax
	movzwl %ax, %eax
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:1077
		if other < self { other } else { self }
	cmpl %r10d, %r8d
	cmovbl %r8d, %r10d
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:1038
		if other < self { self } else { other }
	cmpl %r11d, %r8d
	cmoval %r8d, %r11d
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:1077
		if other < self { other } else { self }
	cmpl %r12d, %ebx
	movl %r10d, 440(%rsp)
	movl %r10d, 396(%rsp)
	cmovbl %ebx, %r12d
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:1038
		if other < self { self } else { other }
	cmpl %r15d, %ebx
	movl %r11d, 556(%rsp)
	cmoval %ebx, %r15d
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:1077
		if other < self { other } else { self }
	cmpl %r13d, %edx
	movl %r12d, 540(%rsp)
	movl %r12d, 392(%rsp)
	cmovbl %edx, %r13d
	movl 528(%rsp), %edx
	movl %r15d, 536(%rsp)
	movl %r13d, 532(%rsp)
	movl %r13d, 384(%rsp)
	movq %r8, %r13
	movq %rbp, %r8
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:1038
		if other < self { self } else { other }
	cmpl %edx, %eax
	cmoval %eax, %edx
	movl %edx, %eax
	movq %rax, 496(%rsp)
	movl %r15d, %eax
	movq %rdi, %r15
	movq %rsi, %rdi
	movq %r9, %rsi
	movq %rax, 504(%rsp)
	movl %r11d, %eax
	movl %edx, 528(%rsp)
	movq %rax, 512(%rsp)
	jmp .LBB226_197
	.p2align	4
.LBB226_196:
	movq 16(%rsp), %r13
.LBB226_197:
	movl %edi, %edx
	movl %esi, %r10d
	andl $1, %edx
	shrl $15, %edi
	andl $1, %r10d
	shrl $15, %esi
		// crates/impact_voxel/src/object/extraction.rs:1044
		if on_lower_x_face {
	testl %r13d, %r13d
	je .LBB226_201
	movq 56(%rsp), %r12
	movq 576(%rsp), %rbp
	cmpl $15, %r13d
	jne .LBB226_200
	movq 968(%rsp), %r11
	movq 960(%rsp), %rax
	movq 16(%rsp), %r13
	movq %rsi, %r9
	movq %rdi, %rsi
	movq %r15, %rdi
		// crates/impact_voxel/src/object.rs:2938
		self.0[0][1] += count;
	addq %r15, %r11
	movq %rax, %r15
	addq %r8, %r15
	movq %r11, 968(%rsp)
	movq %r11, 664(%rsp)
	movq %r15, 960(%rsp)
	movq %r15, 888(%rsp)
	movq %rdi, %r15
	movq %rsi, %rdi
	movq %r9, %rsi
.LBB226_200:
		// crates/impact_voxel/src/object/extraction.rs:1051
		if on_lower_y_face {
	testl %ebx, %ebx
	jne .LBB226_202
	jmp .LBB226_163
	.p2align	4
.LBB226_201:
	movq 976(%rsp), %r12
	movq 984(%rsp), %r11
	movq 576(%rsp), %rbp
		// crates/impact_voxel/src/object.rs:2934
		self.0[0][0] += count;
	addq %r8, %r12
	addq %r15, %r11
	movq %r12, 976(%rsp)
	movq %r12, 896(%rsp)
	movq 56(%rsp), %r12
	movq %r11, 984(%rsp)
	movq %r11, 672(%rsp)
		// crates/impact_voxel/src/object/extraction.rs:1051
		if on_lower_y_face {
	testl %ebx, %ebx
	je .LBB226_163
.LBB226_202:
	cmpl $15, %ebx
	jne .LBB226_164
	movq 952(%rsp), %r9
		// crates/impact_voxel/src/object.rs:2946
		self.0[1][1] += count;
	addq %r15, 680(%rsp)
	addq %r8, %r9
	movq %r9, 952(%rsp)
	movq %r9, 904(%rsp)
	jmp .LBB226_164
	.p2align	4
.LBB226_204:
	movl 172(%rsp), %r10d
	movq 928(%rsp), %r11
	movq 920(%rsp), %r14
	movl 388(%rsp), %eax
		// crates/impact_voxel/src/object/extraction.rs:1064
		.all(|(&lower, &upper)| lower > upper);
	cmpl 696(%rsp), %eax
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/iter/traits/iterator.rs:2494
		accum = f(accum, x)?;
	jbe .LBB226_247
	cmpl %r14d, 380(%rsp)
	jbe .LBB226_247
	cmpl %r11d, %r10d
	jbe .LBB226_247
	movq 856(%rsp), %rcx
	movq 568(%rsp), %rsi
	movq 1144(%rsp), %rdx
	movl $49, %eax
.LBB226_208:
		// crates/impact_voxel/src/lib.rs:253
		self.encoded > Self::VOID_LIMIT
	cmpb $101, -48(%rdx,%rax)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/iter/macros.rs:315
		if !f(x) {
	jl .LBB226_226
		// crates/impact_voxel/src/lib.rs:253
		self.encoded > Self::VOID_LIMIT
	cmpb $101, -45(%rdx,%rax)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/iter/macros.rs:315
		if !f(x) {
	jl .LBB226_226
		// crates/impact_voxel/src/lib.rs:253
		self.encoded > Self::VOID_LIMIT
	cmpb $101, -42(%rdx,%rax)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/iter/macros.rs:315
		if !f(x) {
	jl .LBB226_226
		// crates/impact_voxel/src/lib.rs:253
		self.encoded > Self::VOID_LIMIT
	cmpb $101, -39(%rdx,%rax)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/iter/macros.rs:315
		if !f(x) {
	jl .LBB226_226
		// crates/impact_voxel/src/lib.rs:253
		self.encoded > Self::VOID_LIMIT
	cmpb $101, -36(%rdx,%rax)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/iter/macros.rs:315
		if !f(x) {
	jl .LBB226_226
		// crates/impact_voxel/src/lib.rs:253
		self.encoded > Self::VOID_LIMIT
	cmpb $101, -33(%rdx,%rax)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/iter/macros.rs:315
		if !f(x) {
	jl .LBB226_226
		// crates/impact_voxel/src/lib.rs:253
		self.encoded > Self::VOID_LIMIT
	cmpb $101, -30(%rdx,%rax)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/iter/macros.rs:315
		if !f(x) {
	jl .LBB226_226
		// crates/impact_voxel/src/lib.rs:253
		self.encoded > Self::VOID_LIMIT
	cmpb $101, -27(%rdx,%rax)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/iter/macros.rs:315
		if !f(x) {
	jl .LBB226_226
		// crates/impact_voxel/src/lib.rs:253
		self.encoded > Self::VOID_LIMIT
	cmpb $101, -24(%rdx,%rax)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/iter/macros.rs:315
		if !f(x) {
	jl .LBB226_226
		// crates/impact_voxel/src/lib.rs:253
		self.encoded > Self::VOID_LIMIT
	cmpb $101, -21(%rdx,%rax)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/iter/macros.rs:315
		if !f(x) {
	jl .LBB226_226
		// crates/impact_voxel/src/lib.rs:253
		self.encoded > Self::VOID_LIMIT
	cmpb $101, -18(%rdx,%rax)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/iter/macros.rs:315
		if !f(x) {
	jl .LBB226_226
		// crates/impact_voxel/src/lib.rs:253
		self.encoded > Self::VOID_LIMIT
	cmpb $101, -15(%rdx,%rax)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/iter/macros.rs:315
		if !f(x) {
	jl .LBB226_226
		// crates/impact_voxel/src/lib.rs:253
		self.encoded > Self::VOID_LIMIT
	cmpb $101, -12(%rdx,%rax)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/iter/macros.rs:315
		if !f(x) {
	jl .LBB226_226
		// crates/impact_voxel/src/lib.rs:253
		self.encoded > Self::VOID_LIMIT
	cmpb $101, -9(%rdx,%rax)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/iter/macros.rs:315
		if !f(x) {
	jl .LBB226_226
		// crates/impact_voxel/src/lib.rs:253
		self.encoded > Self::VOID_LIMIT
	cmpb $101, -6(%rdx,%rax)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/iter/macros.rs:315
		if !f(x) {
	jl .LBB226_226
		// crates/impact_voxel/src/lib.rs:253
		self.encoded > Self::VOID_LIMIT
	cmpb $101, -3(%rdx,%rax)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/iter/macros.rs:315
		if !f(x) {
	jl .LBB226_226
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/non_null.rs:1714
		self.as_ptr() == other.as_ptr()
	cmpq $12289, %rax
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/iter/macros.rs:180
		if ptr == crate::intrinsics::transmute::<$ptr, NonNull<T>>(end_or_len) {
	je .LBB226_345
		// crates/impact_voxel/src/lib.rs:253
		self.encoded > Self::VOID_LIMIT
	cmpb $100, (%rdx,%rax)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/iter/macros.rs:315
		if !f(x) {
	leaq 51(%rax), %rax
	jg .LBB226_208
.LBB226_226:
	movq 672(%rsp), %rdi
	movq 664(%rsp), %r8
		// crates/impact_voxel/src/object.rs:2989
		if non_empty_count == full_face_count {
	xorl %eax, %eax
	movq 688(%rsp), %r10
	movq 680(%rsp), %r11
	movq 56(%rsp), %r14
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
	testq %r15, %r15
	setne %r10b
	shll $9, %r10d
	cmpq $256, %r15
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
		// crates/impact_voxel/src/object/extraction.rs:1074
		chunk.face_distributions =
	movw %di, 13(%rcx)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/array/mod.rs:162
		}
	orq %r9, %rax
		// crates/impact_voxel/src/object/extraction.rs:1074
		chunk.face_distributions =
	movl %eax, 9(%rcx)
		// crates/impact_voxel/src/object.rs:163
		bitflags! {
	orb $64, 8(%rcx)
	.cfi_escape 0x2e, 0x00
	movq 8(%rsp), %rax
	movq 632(%rsp), %rdi
	movq 640(%rsp), %rdx
	movq (%rsp), %r13
	movq 320(%rsp), %r8
	movq %rax, 16(%rsp)
		// crates/impact_voxel/src/object/extraction.rs:1081
		.update_local_connected_regions_for_chunk_with_single_region(
	callq *<impact_voxel::object::split_detection::SplitDetector>::update_local_connected_regions_for_chunk_with_single_region@GOTPCREL(%rip)
	jmp .LBB226_248
	.p2align	4
.LBB226_227:
	movq 88(%rsp), %rax
.LBB226_228:
	movq 352(%rsp), %r12
		// crates/impact_voxel/src/object/extraction.rs:779
		match chunk {
	cmpb $2, %r10b
	je .LBB226_233
		// crates/impact_voxel/src/voxel_types.rs:136
		self.0 as usize
	movzbl %r12b, %edi
		// crates/impact_voxel/src/object/inertia.rs:709
		let density = voxel_type_densities[voxel_type.idx()];
	cmpq %rdi, 248(%rsp)
	jbe .LBB226_430
		// crates/impact_voxel/src/object/inertia.rs:713
		let xl = (chunk_indices[0] as f32) * chunk_extent;
	vcvtuqq2psx 208(%rsp), %xmm1
	vmovaps 1856(%rsp), %xmm2
	movq 592(%rsp), %rax
	movq 160(%rsp), %rcx
		// crates/impact_voxel/src/object/inertia.rs:709
		let density = voxel_type_densities[voxel_type.idx()];
	vmovss (%rax,%rdi,4), %xmm0
	movq 368(%rsp), %rax
		// crates/impact_voxel/src/object/inertia.rs:749
		((1.0 / 3.0) * chunk_extent_pow_2 * density) * (h3_sub_l3.yxx() + h3_sub_l3.zzy());
	vmulss 708(%rsp), %xmm0, %xmm7
		// crates/impact_voxel/src/object/inertia.rs:746
		let mass = chunk_extent_pow_3 * density;
	vmulss 716(%rsp), %xmm0, %xmm9
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
	vmovaps 1872(%rsp), %xmm4
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
	vmulss 712(%rsp), %xmm0, %xmm6
		// crates/impact_voxel/src/object/inertia.rs:751
		(0.25 * chunk_extent * density) * h2_sub_l2.component_mul(&h2_sub_l2.yzx());
	vmulss 704(%rsp), %xmm0, %xmm0
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
	jne .LBB226_232
	.cfi_escape 0x2e, 0x00
	movq 8(%rsp), %rax
	movq (%rsp), %r13
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/vec/mod.rs:1041
		self.buf.grow_one();
	leaq 96(%rsp), %rdi
	movq %rax, 16(%rsp)
	vzeroupper
	callq *<alloc::raw_vec::RawVec<impact_voxel::voxel_types::FixedVoxelMaterialProperties>>::grow_one@GOTPCREL(%rip)
.LBB226_232:
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/raw_vec/mod.rs:614
		self.ptr.cast().as_non_null_ptr()
	movq 104(%rsp), %rdx
	movq 240(%rsp), %rcx
		// crates/impact_voxel/src/object/extraction.rs:843
		poly_chunks.push(VoxelChunk::Uniform(chunk));
	shlq $32, %r12
	movl %ebp, %eax
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mut_ptr.rs:961
		unsafe { intrinsics::offset(self, count) }
	movq %rbx, %rsi
	shlq $4, %rsi
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/vec/mod.rs:1046
		self.len = len + 1;
	incq %rbx
	incq %rbp
	movb $63, %r15b
		// crates/impact_voxel/src/object/extraction.rs:843
		poly_chunks.push(VoxelChunk::Uniform(chunk));
	orq %r12, %rax
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mod.rs:1933
		intrinsics::write_via_move(dst, src)
	movq %rax, (%rdx,%rsi)
	movb $4, 9(%rdx,%rsi)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/vec/mod.rs:1046
		self.len = len + 1;
	movq %rbx, 112(%rsp)
		// crates/impact_voxel/src/object/extraction.rs:844
		self.chunks[chunk_idx] = VoxelChunk::Void;
	movb $3, (%rcx)
		// crates/impact_voxel/src/object/extraction.rs:847
		}
	jmp .LBB226_313
	.p2align	4
.LBB226_233:
	movq 64(%rsp), %rdi
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
	ja .LBB226_413
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/index.rs:101
		let ptr = unsafe { crate::intrinsics::offset(ptr, offset) };
	leaq (%rdi,%rdi,2), %rbx
	addq 32(%rax), %rbx
	.cfi_escape 0x2e, 0x00
	vmovaps 272(%rsp), %xmm0
	movq 8(%rsp), %rax
	movq 592(%rsp), %rcx
	movq 248(%rsp), %r8
	movq (%rsp), %r13
		// crates/impact_voxel/src/object/inertia.rs:433
		compute_moments_for_non_uniform_chunk(
	movl $4096, %edx
	leaq 1312(%rsp), %rdi
	leaq 208(%rsp), %r9
	movq %rbx, %rsi
	movq %rax, 16(%rsp)
	vzeroupper
	callq impact_voxel::object::inertia::compute_moments_for_non_uniform_chunk
	movq 368(%rsp), %rax
		// crates/impact_voxel/src/object/inertia.rs:432
		let (chunk_mass, chunk_moments, chunk_moments_of_inertia, chunk_products_of_inertia) =
	vmovss 1344(%rsp), %xmm0
	vmovaps 1312(%rsp), %xmm1
	vmovaps 1328(%rsp), %xmm2
	vmovaps 1360(%rsp), %xmm3
	movq 160(%rsp), %rcx
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
	movq 400(%rsp), %rax
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/vec/mod.rs:1471
		self.buf.reserve(self.len, additional);
	movq 416(%rsp), %r14
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/num/uint_macros.rs:2621
		intrinsics::wrapping_sub(self, rhs)
	subq %r14, %rax
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/raw_vec/mod.rs:767
		additional > self.capacity(elem_layout.size()).wrapping_sub(len)
	cmpq $4095, %rax
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/raw_vec/mod.rs:673
		if self.needs_to_grow(len, additional, elem_layout) {
	jbe .LBB226_348
.LBB226_236:
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mut_ptr.rs:961
		unsafe { intrinsics::offset(self, count) }
	leaq (%r14,%r14,2), %rdi
	addq 408(%rsp), %rdi
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
	movq %r14, 416(%rsp)
	.p2align	4
.LBB226_237:
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
	jne .LBB226_237
	movq 240(%rsp), %rax
	movq (%rsp), %r13
		// crates/impact_voxel/src/object/extraction.rs:797
		self.chunks[chunk_idx] = VoxelChunk::Void;
	movb $3, (%rax)
		// crates/impact_voxel/src/object/extraction.rs:799
		let face_distributions = chunk.face_distributions;
	movl 464(%rsp), %ecx
	movzbl 468(%rsp), %eax
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/vec/mod.rs:3054
		let len = self.len;
	movq 112(%rsp), %rsi
		// crates/impact_voxel/src/object/extraction.rs:799
		let face_distributions = chunk.face_distributions;
	movl %ecx, 1696(%rsp)
		// crates/impact_voxel/src/object/extraction.rs:801
		let poly_chunk = NonUniformVoxelChunk {
	movl %ecx, 472(%rsp)
		// crates/impact_voxel/src/object/extraction.rs:812
		non_uniform_chunks_inside.push((chunk, poly_chunks.len()));
	movl %ecx, 1312(%rsp)
	movzwl 468(%rsp), %ecx
		// crates/impact_voxel/src/object/extraction.rs:799
		let face_distributions = chunk.face_distributions;
	movb %al, 1700(%rsp)
		// crates/impact_voxel/src/object/extraction.rs:801
		let poly_chunk = NonUniformVoxelChunk {
	movb %al, 476(%rsp)
	movq %rsi, 152(%rsp)
		// crates/impact_voxel/src/object/extraction.rs:812
		non_uniform_chunks_inside.push((chunk, poly_chunks.len()));
	movw %cx, 1316(%rsp)
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/allocator-api2-0.2.21/src/stable/vec/mod.rs:1793
		if self.len == self.buf.capacity() {
	cmpq %r13, 560(%rsp)
	jne .LBB226_264
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/num/uint_macros.rs:896
		if intrinsics::unlikely(intrinsics::add_with_overflow(self, rhs).1) {
	cmpq $-1, %r13
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/intrinsics/mod.rs:459
		if b {
	je .LBB226_414
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
	leaq (%rcx,%rcx,2), %rdx
	movabsq $384307168202282325, %rcx
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/allocator-api2-0.2.21/src/stable/raw_vec.rs:333
		if mem::size_of::<T>() == 0 || self.cap == 0 {
	testq %r13, %r13
	je .LBB226_269
	movq (%rsp), %r13
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/alloc/layout.rs:573
		if element_size != 0 && n > Layout::max_size_for_alignment(alignment) / element_size {
	cmpq %rcx, %rax
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:966
		match self {
	ja .LBB226_414
		// crates/impact_alloc/src/arena.rs:169
		unsafe { &*self.arena_ptr }
	movq 72(%rsp), %rdi
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/alloc/layout.rs:573
		if element_size != 0 && n > Layout::max_size_for_alignment(alignment) / element_size {
	leaq (,%r13,8), %rax
	leaq (%rax,%rax,2), %rcx
	movq %rcx, 56(%rsp)
	movq 16(%rdi), %rax
	movq 32(%rax), %rbx
	cmpq 32(%rsp), %rbx
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:2355
		if align_is_compatible && self.is_last_allocation(ptr) {
	je .LBB226_283
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:1898
		let start = footer.data.as_ptr();
	movq (%rax), %rcx
.LBB226_244:
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mut_ptr.rs:470
		unsafe { intrinsics::arith_offset(self, count) as *mut T }
	andq $-8, %rbx
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:1952
		if aligned_ptr < start || aligned_size > capacity {
	cmpq %rcx, %rbx
	jb .LBB226_277
	movq %rbx, %rsi
	subq %rcx, %rsi
	cmpq %rsi, %rdx
	ja .LBB226_277
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mut_ptr.rs:470
		unsafe { intrinsics::arith_offset(self, count) as *mut T }
	subq %rdx, %rbx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/mem/mod.rs:931
		crate::intrinsics::write_via_move(dest, src);
	movq %rbx, 32(%rax)
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:1880
		if let Some(p) = self.try_alloc_layout_fast(layout) {
	jmp .LBB226_279
.LBB226_247:
	movq 672(%rsp), %rdx
	movq 664(%rsp), %rsi
		// crates/impact_voxel/src/object.rs:2989
		if non_empty_count == full_face_count {
	xorl %eax, %eax
	movq 680(%rsp), %r9
	movl $256, %r8d
	movl $1, %edi
	testq %rdx, %rdx
	setne %al
	xorl %ecx, %ecx
	testq %rsi, %rsi
	setne %cl
	shll $9, %ecx
	cmpq $256, %rsi
	movq 688(%rsp), %rsi
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
	movq 56(%rsp), %r9
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
	testq %r15, %r15
	setne %sil
	shll $9, %esi
	cmpq $256, %r15
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
	movq 856(%rsp), %rcx
		// crates/impact_voxel/src/object/extraction.rs:1074
		chunk.face_distributions =
	movl %eax, 9(%rcx)
		// crates/impact_voxel/src/object/extraction.rs:1090
		lower_occupied_voxels[dim] as usize
	movl 388(%rsp), %eax
		// crates/impact_voxel/src/object/extraction.rs:1074
		chunk.face_distributions =
	movw %si, 13(%rcx)
		// crates/impact_voxel/src/object.rs:163
		bitflags! {
	andb $-65, 8(%rcx)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/array/mod.rs:114
		try_from_fn(NeverShortCircuit::wrap_mut_1(f)).0
	movq %rax, 1312(%rsp)
	movq 696(%rsp), %rax
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:1077
		if other < self { other } else { self }
	cmpl $15, %eax
	cmovael %edx, %eax
	incl %eax
	cmpl $15, %r14d
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/array/mod.rs:114
		try_from_fn(NeverShortCircuit::wrap_mut_1(f)).0
	movq %rax, 1320(%rsp)
		// crates/impact_voxel/src/object/extraction.rs:1090
		lower_occupied_voxels[dim] as usize
	movl 380(%rsp), %eax
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:1077
		if other < self { other } else { self }
	cmovael %edx, %r14d
	incl %r14d
	cmpl $15, %r11d
	cmovael %edx, %r11d
	incl %r11d
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/array/mod.rs:114
		try_from_fn(NeverShortCircuit::wrap_mut_1(f)).0
	movq %rax, 1328(%rsp)
		// crates/impact_voxel/src/object/extraction.rs:1090
		lower_occupied_voxels[dim] as usize
	movl %r10d, %eax
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/array/mod.rs:114
		try_from_fn(NeverShortCircuit::wrap_mut_1(f)).0
	movq %r14, 1336(%rsp)
	movq %rax, 1344(%rsp)
	movq %r11, 1352(%rsp)
	.cfi_escape 0x2e, 0x00
	movq 8(%rsp), %rax
	movq 632(%rsp), %rdi
	movq 568(%rsp), %rsi
	movq 640(%rsp), %rdx
	movq (%rsp), %r13
		// crates/impact_voxel/src/object/extraction.rs:1094
		.update_local_connected_regions_within_occupied_ranges_for_chunk(
	leaq 1312(%rsp), %r9
	movq 320(%rsp), %r8
	movq %rax, 16(%rsp)
	callq *<impact_voxel::object::split_detection::SplitDetector>::update_local_connected_regions_within_occupied_ranges_for_chunk@GOTPCREL(%rip)
.LBB226_248:
	movq 1152(%rsp), %rcx
	movl 396(%rsp), %eax
	xorl %edx, %edx
		// crates/impact_voxel/src/object/extraction.rs:1106
		.all(|(&lower, &upper)| lower > upper);
	cmpl 512(%rsp), %eax
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/iter/traits/iterator.rs:2494
		accum = f(accum, x)?;
	jbe .LBB226_255
	movl 392(%rsp), %eax
	cmpl 504(%rsp), %eax
	jbe .LBB226_255
	movl 384(%rsp), %eax
	cmpl 496(%rsp), %eax
	jbe .LBB226_255
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/iter/macros.rs:314
		while let Some(x) = self.next() {
	subq 1128(%rsp), %rcx
	movq 1136(%rsp), %rax
	addq $12288, %rcx
	.p2align	4
.LBB226_252:
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/non_null.rs:1714
		self.as_ptr() == other.as_ptr()
	testq %rcx, %rcx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/iter/macros.rs:180
		if ptr == crate::intrinsics::transmute::<$ptr, NonNull<T>>(end_or_len) {
	je .LBB226_266
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
	jg .LBB226_252
	movb $64, %dl
.LBB226_255:
	movq 896(%rsp), %rax
	movq 888(%rsp), %rcx
	movl %edx, 56(%rsp)
		// crates/impact_voxel/src/object.rs:2989
		if non_empty_count == full_face_count {
	xorl %edx, %edx
	testq %rax, %rax
	setne %dl
	xorl %esi, %esi
	testq %rcx, %rcx
	setne %sil
	shll $9, %esi
	cmpq $256, %rcx
	movl $256, %ecx
	cmovel %ecx, %esi
	addl %edx, %edx
	cmpq $256, %rax
	movl $1, %eax
	cmovel %eax, %edx
	movl %esi, 264(%rsp)
	movq 904(%rsp), %rsi
	xorl %r14d, %r14d
	movl %edx, 232(%rsp)
	movq 912(%rsp), %rdx
	testq %rdx, %rdx
	setne %r14b
	xorl %edi, %edi
	testq %rsi, %rsi
	setne %dil
	shll $9, %edi
	cmpq $256, %rsi
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/vec/mod.rs:3054
		let len = self.len;
	movq 112(%rsp), %rsi
		// crates/impact_voxel/src/object.rs:2989
		if non_empty_count == full_face_count {
	cmovel %ecx, %edi
	addl %r14d, %r14d
	cmpq $256, %rdx
	cmovel %eax, %r14d
	xorl %edx, %edx
	testq %rbx, %rbx
	movl %edi, 352(%rsp)
	setne %dl
	xorl %r15d, %r15d
	testq %r12, %r12
	setne %r15b
	shll $9, %r15d
	cmpq $256, %r12
	movq %rsi, 152(%rsp)
	cmovel %ecx, %r15d
	movq 512(%rsp), %rcx
	addl %edx, %edx
	cmpq $256, %rbx
	cmovel %eax, %edx
	movl $15, %eax
	movl %edx, 344(%rsp)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:1077
		if other < self { other } else { self }
	cmpl $15, %ecx
	cmovael %eax, %ecx
	movq %rcx, 512(%rsp)
	movq 504(%rsp), %rcx
	cmpl $15, %ecx
	cmovael %eax, %ecx
	movq %rcx, 504(%rsp)
	movq 496(%rsp), %rcx
	cmpl $15, %ecx
	cmovael %eax, %ecx
	movq 8(%rsp), %rax
	movq %rcx, 496(%rsp)
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/allocator-api2-0.2.21/src/stable/vec/mod.rs:1793
		if self.len == self.buf.capacity() {
	cmpq %rax, 296(%rsp)
	jne .LBB226_265
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/num/uint_macros.rs:896
		if intrinsics::unlikely(intrinsics::add_with_overflow(self, rhs).1) {
	cmpq $-1, %rax
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/intrinsics/mod.rs:459
		if b {
	je .LBB226_415
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/num/uint_macros.rs:896
		if intrinsics::unlikely(intrinsics::add_with_overflow(self, rhs).1) {
	leaq 1(%rax), %rcx
	movq %rax, %rdi
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
	movq %rcx, 64(%rsp)
	movabsq $164703072086692425, %rcx
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/allocator-api2-0.2.21/src/stable/raw_vec.rs:333
		if mem::size_of::<T>() == 0 || self.cap == 0 {
	testq %rdi, %rdi
	je .LBB226_273
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/alloc/layout.rs:573
		if element_size != 0 && n > Layout::max_size_for_alignment(alignment) / element_size {
	cmpq %rcx, %rax
	movq 8(%rsp), %rax
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:966
		match self {
	ja .LBB226_415
		// crates/impact_alloc/src/arena.rs:169
		unsafe { &*self.arena_ptr }
	movq 72(%rsp), %rdi
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/alloc/layout.rs:573
		if element_size != 0 && n > Layout::max_size_for_alignment(alignment) / element_size {
	imulq $56, %rax, %rcx
	movq %rcx, 240(%rsp)
	movq 16(%rdi), %rax
	movq 32(%rax), %rbx
	cmpq 128(%rsp), %rbx
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:2355
		if align_is_compatible && self.is_last_allocation(ptr) {
	je .LBB226_287
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:1898
		let start = footer.data.as_ptr();
	movq (%rax), %rcx
.LBB226_261:
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mut_ptr.rs:470
		unsafe { intrinsics::arith_offset(self, count) as *mut T }
	andq $-8, %rbx
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:1952
		if aligned_ptr < start || aligned_size > capacity {
	cmpq %rcx, %rbx
	jb .LBB226_280
	movq %rbx, %rsi
	subq %rcx, %rsi
	cmpq %rsi, %rdx
	ja .LBB226_280
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mut_ptr.rs:470
		unsafe { intrinsics::arith_offset(self, count) as *mut T }
	subq %rdx, %rbx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/mem/mod.rs:931
		crate::intrinsics::write_via_move(dest, src);
	movq %rbx, 32(%rax)
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:1880
		if let Some(p) = self.try_alloc_layout_fast(layout) {
	jmp .LBB226_282
.LBB226_264:
	movq 32(%rsp), %rbx
	movq %r13, %r14
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/allocator-api2-0.2.21/src/stable/vec/mod.rs:1793
		if self.len == self.buf.capacity() {
	jmp .LBB226_292
.LBB226_265:
	movq 128(%rsp), %rbx
	movq %rax, 64(%rsp)
	jmp .LBB226_309
.LBB226_266:
	movq 648(%rsp), %rax
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/vec/mod.rs:1037
		let len = self.len;
	movq 112(%rsp), %rbx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/vec/mod.rs:1827
		self.len = len;
	movq %rax, 416(%rsp)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/vec/mod.rs:1040
		if len == self.buf.capacity() {
	cmpq 96(%rsp), %rbx
	jne .LBB226_268
	.cfi_escape 0x2e, 0x00
	movq 8(%rsp), %rax
	movq (%rsp), %r13
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/vec/mod.rs:1041
		self.buf.grow_one();
	leaq 96(%rsp), %rdi
	movq %rax, 16(%rsp)
	callq *<alloc::raw_vec::RawVec<impact_voxel::voxel_types::FixedVoxelMaterialProperties>>::grow_one@GOTPCREL(%rip)
.LBB226_268:
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
	jmp .LBB226_312
.LBB226_269:
	movq (%rsp), %r13
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/alloc/layout.rs:573
		if element_size != 0 && n > Layout::max_size_for_alignment(alignment) / element_size {
	cmpq %rcx, %rax
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:966
		match self {
	ja .LBB226_414
		// crates/impact_alloc/src/arena.rs:169
		unsafe { &*self.arena_ptr }
	movq 72(%rsp), %rdi
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
	cmpq $96, %rcx
	setb %cl
	orb %sil, %cl
	je .LBB226_291
	.cfi_escape 0x2e, 0x00
	movq 8(%rsp), %rax
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:1883
		self.alloc_layout_slow(layout).ok_or(AllocErr)
	movl $8, %esi
	xorl %r13d, %r13d
	movq %rdx, %r15
	movq %rax, 16(%rsp)
	callq *<bumpalo::Bump>::alloc_layout_slow@GOTPCREL(%rip)
	movq %rax, %rbx
	movq %r15, %rdx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:966
		match self {
	testq %rax, %rax
	jne .LBB226_292
	jmp .LBB226_436
.LBB226_273:
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/alloc/layout.rs:573
		if element_size != 0 && n > Layout::max_size_for_alignment(alignment) / element_size {
	cmpq %rcx, %rax
	movq 8(%rsp), %rax
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:966
		match self {
	ja .LBB226_415
		// crates/impact_alloc/src/arena.rs:169
		unsafe { &*self.arena_ptr }
	movq 72(%rsp), %rdi
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
	je .LBB226_308
	.cfi_escape 0x2e, 0x00
	movq (%rsp), %r13
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:1883
		self.alloc_layout_slow(layout).ok_or(AllocErr)
	movl $8, %esi
	movq $0, 16(%rsp)
	movq %rdx, %r12
	callq *<bumpalo::Bump>::alloc_layout_slow@GOTPCREL(%rip)
	movq %rax, %rbx
	movq %r12, %rdx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:966
		match self {
	testq %rax, %rax
	jne .LBB226_309
	jmp .LBB226_437
.LBB226_277:
	.cfi_escape 0x2e, 0x00
	movq 8(%rsp), %rax
	movq (%rsp), %r13
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:1883
		self.alloc_layout_slow(layout).ok_or(AllocErr)
	movl $8, %esi
	movq %rdx, %r15
	movq %rax, 16(%rsp)
	callq *<bumpalo::Bump>::alloc_layout_slow@GOTPCREL(%rip)
	movq %rax, %rbx
	movq %r15, %rdx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/option.rs:1335
		match self {
	testq %rax, %rax
	je .LBB226_436
.LBB226_279:
	.cfi_escape 0x2e, 0x00
	movq 32(%rsp), %rsi
	movq 56(%rsp), %rdx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mod.rs:551
		unsafe { crate::intrinsics::copy_nonoverlapping(src, dst, count) }
	movq %rbx, %rdi
	callq *memcpy@GOTPCREL(%rip)
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:2371
		}
	jmp .LBB226_292
.LBB226_280:
	.cfi_escape 0x2e, 0x00
	movq 8(%rsp), %rax
	movq (%rsp), %r13
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:1883
		self.alloc_layout_slow(layout).ok_or(AllocErr)
	movl $8, %esi
	movq %rdx, %r12
	movq %rax, 16(%rsp)
	callq *<bumpalo::Bump>::alloc_layout_slow@GOTPCREL(%rip)
	movq %rax, %rbx
	movq %r12, %rdx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/option.rs:1335
		match self {
	testq %rax, %rax
	je .LBB226_437
.LBB226_282:
	.cfi_escape 0x2e, 0x00
	movq 128(%rsp), %rsi
	movq 240(%rsp), %rdx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mod.rs:551
		unsafe { crate::intrinsics::copy_nonoverlapping(src, dst, count) }
	movq %rbx, %rdi
	callq *memcpy@GOTPCREL(%rip)
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:2371
		}
	jmp .LBB226_309
.LBB226_283:
	movabsq $9223372036854775793, %rsi
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:2358
		let delta = new_size - old_size;
	movq %rdx, %r8
	subq %rcx, %r8
	leaq 8(%rsi), %rcx
	cmpq %rcx, %r8
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/mem/alignment.rs:137
		if align.is_power_of_two() {
	jae .LBB226_436
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:1898
		let start = footer.data.as_ptr();
	movq (%rax), %rcx
	movq 32(%rsp), %rsi
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
	jb .LBB226_244
	cmpq %rsi, %r8
	ja .LBB226_244
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mut_ptr.rs:470
		unsafe { intrinsics::arith_offset(self, count) as *mut T }
	subq %r8, %r13
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/mem/mod.rs:931
		crate::intrinsics::write_via_move(dest, src);
	movq %r13, 32(%rax)
	.cfi_escape 0x2e, 0x00
	movq 32(%rsp), %rsi
	movq 56(%rsp), %rdx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mod.rs:642
		crate::intrinsics::copy(src, dst, count)
	movq %r13, %rdi
	callq *memmove@GOTPCREL(%rip)
	movq %r13, %rbx
	jmp .LBB226_292
.LBB226_287:
	movabsq $9223372036854775793, %rsi
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:2358
		let delta = new_size - old_size;
	movq %rdx, %r8
	subq %rcx, %r8
	leaq 8(%rsi), %rcx
	cmpq %rcx, %r8
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/mem/alignment.rs:137
		if align.is_power_of_two() {
	jae .LBB226_437
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
	jb .LBB226_261
	cmpq %rsi, %r8
	ja .LBB226_261
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mut_ptr.rs:470
		unsafe { intrinsics::arith_offset(self, count) as *mut T }
	subq %r8, %r13
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/mem/mod.rs:931
		crate::intrinsics::write_via_move(dest, src);
	movq %r13, 32(%rax)
	.cfi_escape 0x2e, 0x00
	movq 128(%rsp), %rsi
	movq 240(%rsp), %rdx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mod.rs:642
		crate::intrinsics::copy(src, dst, count)
	movq %r13, %rdi
	callq *memmove@GOTPCREL(%rip)
	movq %r13, %rbx
	jmp .LBB226_309
.LBB226_291:
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mut_ptr.rs:470
		unsafe { intrinsics::arith_offset(self, count) as *mut T }
	addq $-96, %rbx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/mem/mod.rs:931
		crate::intrinsics::write_via_move(dest, src);
	movq %rbx, 32(%rax)
	.p2align	4
.LBB226_292:
	movq 560(%rsp), %rax
	movq 64(%rsp), %rcx
	movq 152(%rsp), %rdx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mut_ptr.rs:961
		unsafe { intrinsics::offset(self, count) }
	leaq (%rax,%rax,2), %rax
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mod.rs:1933
		intrinsics::write_via_move(dst, src)
	movl %ecx, (%rbx,%rax,8)
	movzbl 204(%rsp), %ecx
	movl %r12d, 4(%rbx,%rax,8)
	movb %cl, 8(%rbx,%rax,8)
	movzbl 172(%rsp), %ecx
	movb %cl, 9(%rbx,%rax,8)
	movl 1312(%rsp), %ecx
	movl %ecx, 10(%rbx,%rax,8)
	movzwl 1316(%rsp), %ecx
	movw %cx, 14(%rbx,%rax,8)
	movq %rdx, 16(%rbx,%rax,8)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/vec/mod.rs:1037
		let len = self.len;
	movq 112(%rsp), %r15
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/vec/mod.rs:1040
		if len == self.buf.capacity() {
	cmpq 96(%rsp), %r15
	jne .LBB226_294
	.cfi_escape 0x2e, 0x00
	movq 8(%rsp), %rax
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/vec/mod.rs:1041
		self.buf.grow_one();
	leaq 96(%rsp), %rdi
	movq %r14, %r13
	movq %rbx, 32(%rsp)
	movq %rax, 16(%rsp)
	callq *<alloc::raw_vec::RawVec<impact_voxel::voxel_types::FixedVoxelMaterialProperties>>::grow_one@GOTPCREL(%rip)
.LBB226_294:
	movzbl 204(%rsp), %edx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/raw_vec/mod.rs:614
		self.ptr.cast().as_non_null_ptr()
	movq 104(%rsp), %rax
	movq 304(%rsp), %rsi
	movzbl 172(%rsp), %edi
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mut_ptr.rs:961
		unsafe { intrinsics::offset(self, count) }
	movq %r15, %rcx
	shlq $4, %rcx
	incq 560(%rsp)
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
	movq %rsi, 304(%rsp)
	movl 472(%rsp), %edx
	movl %edx, 10(%rax,%rcx)
	movzbl 476(%rsp), %edx
	movb %dl, 14(%rax,%rcx)
	leaq 1329(%rsp), %rcx
	movq 488(%rsp), %rdx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/vec/mod.rs:1046
		self.len = len + 1;
	movq %r15, 112(%rsp)
	xorl %r15d, %r15d
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/array/iter.rs:68
		let data: [MaybeUninit<T>; N] = unsafe { transmute_unchecked(self) };
	movzbl 1700(%rsp), %eax
	movb %al, 4(%rcx)
	movl 1696(%rsp), %eax
	movl %eax, (%rcx)
		// crates/impact_voxel/src/object/extraction.rs:818
		face_distributions.into_iter().enumerate()
	movq $3, 1320(%rsp)
	movb %dil, 1328(%rsp)
	xorl %eax, %eax
	xorl %ecx, %ecx
	.p2align	4
.LBB226_295:
	movq %rdx, 488(%rsp)
	movl $3, %edx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ops/index_range.rs:131
		if self.len() > 0 {
	cmpq $3, %rcx
	je .LBB226_298
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mod.rs:1733
		crate::intrinsics::read_via_copy(src)
	movzwl 1328(%rsp,%rcx,2), %edx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/num/uint_macros.rs:971
		intrinsics::unchecked_add(self, rhs)
	incq %rcx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/option.rs:2775
		match self {
	cmpb $-1, %dl
	jne .LBB226_302
	movq %rcx, %rdx
.LBB226_298:
	movl 376(%rsp), %esi
	movq %rdx, %rcx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/option.rs:2790
		None => None,
	orl $255, %esi
	movl %esi, %edx
	movl %edx, 376(%rsp)
		// crates/impact_voxel/src/object/extraction.rs:818
		face_distributions.into_iter().enumerate()
	testb %dl, %dl
	je .LBB226_303
.LBB226_299:
	movzbl %dl, %edx
	cmpl $255, %edx
	je .LBB226_306
	movq 488(%rsp), %rdx
		// crates/impact_voxel/src/object/extraction.rs:821
		invalidated_faces |= Faces::all_lower()[dim];
	movb $4, 994(%rsp)
	movw $513, 992(%rsp)
	cmpq $2, %rdx
	ja .LBB226_418
		// crates/impact_voxel/src/utils.rs:21
		bitflags! {
	orb 992(%rsp,%rdx), %r15b
		// crates/impact_voxel/src/object.rs:150
		#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
	cmpw $256, 376(%rsp)
		// crates/impact_voxel/src/object/extraction.rs:823
		if !distributions[1].is_empty() {
	jb .LBB226_295
	jmp .LBB226_304
	.p2align	4
.LBB226_302:
	movq %rax, 488(%rsp)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/iter/adapters/enumerate.rs:82
		self.count += 1;
	incq %rax
	movl %edx, 376(%rsp)
		// crates/impact_voxel/src/object/extraction.rs:818
		face_distributions.into_iter().enumerate()
	testb %dl, %dl
	jne .LBB226_299
.LBB226_303:
	movq 488(%rsp), %rdx
		// crates/impact_voxel/src/object.rs:150
		#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
	cmpw $256, 376(%rsp)
		// crates/impact_voxel/src/object/extraction.rs:823
		if !distributions[1].is_empty() {
	jb .LBB226_295
.LBB226_304:
		// crates/impact_voxel/src/object/extraction.rs:824
		invalidated_faces |= Faces::all_upper()[dim];
	movb $32, 994(%rsp)
	movw $4104, 992(%rsp)
	cmpq $2, %rdx
	ja .LBB226_419
		// crates/impact_voxel/src/utils.rs:21
		bitflags! {
	orb 992(%rsp,%rdx), %r15b
		// crates/impact_voxel/src/object/extraction.rs:823
		if !distributions[1].is_empty() {
	jmp .LBB226_295
	.p2align	4
.LBB226_306:
		// crates/impact_voxel/src/utils.rs:21
		bitflags! {
	testb %r15b, %r15b
		// crates/impact_voxel/src/object/extraction.rs:1144
		if !invalidated_faces.is_empty() {
	je .LBB226_331
	movq %r14, (%rsp)
	movq %rbx, 32(%rsp)
		// crates/impact_voxel/src/utils.rs:21
		bitflags! {
	testb $1, %r15b
		// crates/impact_voxel/src/object/extraction.rs:1152
		if invalidated_faces.contains(Faces::all_lower()[dim.idx()])
	jne .LBB226_313
	jmp .LBB226_315
.LBB226_308:
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mut_ptr.rs:470
		unsafe { intrinsics::arith_offset(self, count) as *mut T }
	addq $-224, %rbx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/mem/mod.rs:931
		crate::intrinsics::write_via_move(dest, src);
	movq %rbx, 32(%rax)
	.p2align	4
.LBB226_309:
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mut_ptr.rs:961
		unsafe { intrinsics::offset(self, count) }
	imulq $56, 296(%rsp), %rsi
	movq 512(%rsp), %r8
	movq 504(%rsp), %rdi
	movq 496(%rsp), %r12
	movq 152(%rsp), %r9
	movl 396(%rsp), %eax
	movl 392(%rsp), %ecx
	movl 384(%rsp), %edx
	incl %r8d
	incl %edi
	incl %r12d
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mod.rs:1933
		intrinsics::write_via_move(dst, src)
	movq %r9, (%rbx,%rsi)
	movq %rax, 8(%rbx,%rsi)
	movq %r8, 16(%rbx,%rsi)
	movq %rcx, 24(%rbx,%rsi)
	movq %rdi, 32(%rbx,%rsi)
	movq %rdx, 40(%rbx,%rsi)
	movq %r12, 48(%rbx,%rsi)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/vec/mod.rs:1037
		let len = self.len;
	movq 112(%rsp), %r12
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/vec/mod.rs:1040
		if len == self.buf.capacity() {
	cmpq 96(%rsp), %r12
	jne .LBB226_311
	.cfi_escape 0x2e, 0x00
	movq 64(%rsp), %rax
	movq (%rsp), %r13
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/vec/mod.rs:1041
		self.buf.grow_one();
	leaq 96(%rsp), %rdi
	movq %rbx, 128(%rsp)
	movq %rax, 16(%rsp)
	callq *<alloc::raw_vec::RawVec<impact_voxel::voxel_types::FixedVoxelMaterialProperties>>::grow_one@GOTPCREL(%rip)
.LBB226_311:
	addl 352(%rsp), %r14d
	movl 232(%rsp), %eax
	addl 344(%rsp), %r15d
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/raw_vec/mod.rs:614
		self.ptr.cast().as_non_null_ptr()
	movq 104(%rsp), %rcx
	movq 304(%rsp), %rsi
	movl 56(%rsp), %edi
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mut_ptr.rs:961
		unsafe { intrinsics::offset(self, count) }
	movq %r12, %rdx
	shlq $4, %rdx
	incq 296(%rsp)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/vec/mod.rs:1046
		self.len = len + 1;
	incq %r12
	movq %rbx, 128(%rsp)
	addl 264(%rsp), %eax
	shlq $32, %r15
	shll $16, %r14d
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mod.rs:1933
		intrinsics::write_via_move(dst, src)
	movl %esi, (%rcx,%rdx)
	movl $0, 4(%rcx,%rdx)
	movb %dil, 8(%rcx,%rdx)
		// crates/impact_voxel/src/object/extraction.rs:1138
		poly_non_uniform_chunk_count += 1;
	incq %rsi
	orq %r15, %r14
	movzwl %ax, %eax
	movq %rsi, 304(%rsp)
	orq %r14, %rax
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mod.rs:1933
		intrinsics::write_via_move(dst, src)
	movl %eax, 9(%rcx,%rdx)
	shrq $32, %rax
	movw %ax, 13(%rcx,%rdx)
	movq 64(%rsp), %rcx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/vec/mod.rs:1046
		self.len = len + 1;
	movq %r12, 112(%rsp)
	movq %rcx, 8(%rsp)
.LBB226_312:
	movq 88(%rsp), %rax
	movb $63, %r15b
	movq 208(%rax), %rcx
	movq 216(%rax), %rax
	movq %rcx, 232(%rsp)
	movq %rax, 264(%rsp)
.LBB226_313:
		// crates/impact_voxel/src/object/extraction.rs:1153
		&& chunk_indices[dim.idx()] > 0
	movq 208(%rsp), %rax
	testq %rax, %rax
	je .LBB226_316
	movq 216(%rsp), %rsi
		// crates/impact_voxel/src/object/extraction.rs:1156
		lower_chunk_indices[dim.idx()] -= 1;
	decq %rax
		// crates/impact_voxel/src/object.rs:1804
		chunk_indices[0] * self.chunk_idx_strides[0]
	imulq 232(%rsp), %rax
		// crates/impact_voxel/src/object.rs:1805
		+ chunk_indices[1] * self.chunk_idx_strides[1]
	imulq 264(%rsp), %rsi
		// crates/impact_voxel/src/object.rs:1804
		chunk_indices[0] * self.chunk_idx_strides[0]
	addq %rax, %rsi
	addq 224(%rsp), %rsi
	.cfi_escape 0x2e, 0x00
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hashbrown-0.16.1/src/set.rs:1093
		self.map.insert(value, ()).is_none()
	leaq 1040(%rsp), %rdi
	xorl %edx, %edx
	vzeroupper
	callq <hashbrown::map::HashMap<(usize, impact_voxel::utils::Dimension), (), rustc_hash::FxBuildHasher, &impact_alloc::arena::PoolArena>>::insert
	movq (%rsp), %r14
	movq 32(%rsp), %rbx
.LBB226_315:
		// crates/impact_voxel/src/utils.rs:21
		bitflags! {
	testb $8, %r15b
		// crates/impact_voxel/src/object/extraction.rs:1160
		if invalidated_faces.contains(Faces::all_upper()[dim.idx()])
	jne .LBB226_317
	jmp .LBB226_319
	.p2align	4
.LBB226_316:
	movq (%rsp), %r14
	movq 32(%rsp), %rbx
		// crates/impact_voxel/src/utils.rs:21
		bitflags! {
	testb $8, %r15b
		// crates/impact_voxel/src/object/extraction.rs:1160
		if invalidated_faces.contains(Faces::all_upper()[dim.idx()])
	je .LBB226_319
.LBB226_317:
	movq 88(%rsp), %rax
		// crates/impact_voxel/src/object/extraction.rs:1161
		&& chunk_indices[dim.idx()] < self.chunk_counts[dim.idx()] - 1
	movq 184(%rax), %rax
	decq %rax
	cmpq %rax, 208(%rsp)
	jae .LBB226_319
	.cfi_escape 0x2e, 0x00
	movq 320(%rsp), %rsi
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hashbrown-0.16.1/src/set.rs:1093
		self.map.insert(value, ()).is_none()
	leaq 1040(%rsp), %rdi
	xorl %edx, %edx
	movq %r14, (%rsp)
	movq %rbx, 32(%rsp)
	vzeroupper
	callq <hashbrown::map::HashMap<(usize, impact_voxel::utils::Dimension), (), rustc_hash::FxBuildHasher, &impact_alloc::arena::PoolArena>>::insert
.LBB226_319:
	movq 216(%rsp), %r12
		// crates/impact_voxel/src/utils.rs:21
		bitflags! {
	testb $2, %r15b
		// crates/impact_voxel/src/object/extraction.rs:1152
		if invalidated_faces.contains(Faces::all_lower()[dim.idx()])
	je .LBB226_322
	testq %r12, %r12
	je .LBB226_322
	movq 208(%rsp), %rax
		// crates/impact_voxel/src/object/extraction.rs:1156
		lower_chunk_indices[dim.idx()] -= 1;
	leaq -1(%r12), %rsi
		// crates/impact_voxel/src/object.rs:1805
		+ chunk_indices[1] * self.chunk_idx_strides[1]
	imulq 264(%rsp), %rsi
		// crates/impact_voxel/src/object.rs:1804
		chunk_indices[0] * self.chunk_idx_strides[0]
	imulq 232(%rsp), %rax
	addq %rax, %rsi
	addq 224(%rsp), %rsi
	.cfi_escape 0x2e, 0x00
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hashbrown-0.16.1/src/set.rs:1093
		self.map.insert(value, ()).is_none()
	leaq 1040(%rsp), %rdi
	movl $1, %edx
	movq %r14, (%rsp)
	movq %rbx, 32(%rsp)
	vzeroupper
	callq <hashbrown::map::HashMap<(usize, impact_voxel::utils::Dimension), (), rustc_hash::FxBuildHasher, &impact_alloc::arena::PoolArena>>::insert
.LBB226_322:
		// crates/impact_voxel/src/utils.rs:21
		bitflags! {
	testb $16, %r15b
		// crates/impact_voxel/src/object/extraction.rs:1160
		if invalidated_faces.contains(Faces::all_upper()[dim.idx()])
	je .LBB226_325
	movq 88(%rsp), %rax
		// crates/impact_voxel/src/object/extraction.rs:1161
		&& chunk_indices[dim.idx()] < self.chunk_counts[dim.idx()] - 1
	movq 192(%rax), %rax
	decq %rax
	cmpq %rax, %r12
	jae .LBB226_325
	.cfi_escape 0x2e, 0x00
	movq 320(%rsp), %rsi
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hashbrown-0.16.1/src/set.rs:1093
		self.map.insert(value, ()).is_none()
	leaq 1040(%rsp), %rdi
	movl $1, %edx
	movq %r14, (%rsp)
	movq %rbx, 32(%rsp)
	vzeroupper
	callq <hashbrown::map::HashMap<(usize, impact_voxel::utils::Dimension), (), rustc_hash::FxBuildHasher, &impact_alloc::arena::PoolArena>>::insert
.LBB226_325:
	movq 224(%rsp), %r13
		// crates/impact_voxel/src/utils.rs:21
		bitflags! {
	testb $4, %r15b
		// crates/impact_voxel/src/object/extraction.rs:1152
		if invalidated_faces.contains(Faces::all_lower()[dim.idx()])
	je .LBB226_328
	testq %r13, %r13
	je .LBB226_328
	movq 232(%rsp), %rax
		// crates/impact_voxel/src/object.rs:1805
		+ chunk_indices[1] * self.chunk_idx_strides[1]
	imulq 264(%rsp), %r12
		// crates/impact_voxel/src/object.rs:1804
		chunk_indices[0] * self.chunk_idx_strides[0]
	imulq 208(%rsp), %rax
	addq %r13, %r12
	leaq -1(%rax,%r12), %rsi
	.cfi_escape 0x2e, 0x00
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hashbrown-0.16.1/src/set.rs:1093
		self.map.insert(value, ()).is_none()
	leaq 1040(%rsp), %rdi
	movl $2, %edx
	movq %r14, (%rsp)
	movq %rbx, 32(%rsp)
	vzeroupper
	callq <hashbrown::map::HashMap<(usize, impact_voxel::utils::Dimension), (), rustc_hash::FxBuildHasher, &impact_alloc::arena::PoolArena>>::insert
.LBB226_328:
		// crates/impact_voxel/src/utils.rs:21
		bitflags! {
	testb $32, %r15b
		// crates/impact_voxel/src/object/extraction.rs:1160
		if invalidated_faces.contains(Faces::all_upper()[dim.idx()])
	je .LBB226_331
	movq 88(%rsp), %rax
		// crates/impact_voxel/src/object/extraction.rs:1161
		&& chunk_indices[dim.idx()] < self.chunk_counts[dim.idx()] - 1
	movq 200(%rax), %rax
	decq %rax
	cmpq %rax, %r13
	jae .LBB226_331
	.cfi_escape 0x2e, 0x00
	movq 320(%rsp), %rsi
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hashbrown-0.16.1/src/set.rs:1093
		self.map.insert(value, ()).is_none()
	leaq 1040(%rsp), %rdi
	movl $2, %edx
	movq %r14, (%rsp)
	movq %rbx, 32(%rsp)
	vzeroupper
	callq <hashbrown::map::HashMap<(usize, impact_voxel::utils::Dimension), (), rustc_hash::FxBuildHasher, &impact_alloc::arena::PoolArena>>::insert
	movq 8(%rsp), %rax
	movq %r14, %r13
	movq %rax, 16(%rsp)
	jmp .LBB226_332
	.p2align	4
.LBB226_331:
	movq 8(%rsp), %rax
	movq %rbx, 32(%rsp)
	movq %r14, %r13
	movq %rax, 16(%rsp)
.LBB226_332:
	.cfi_escape 0x2e, 0x00
	movq 312(%rsp), %rdi
	leaq 208(%rsp), %rsi
	vzeroupper
	callq <hashbrown::map::HashMap<[usize; 3], (), rustc_hash::FxBuildHasher>>::insert
		// crates/impact_voxel/src/object/extraction.rs:1173
		if chunk_indices[dim] > 0 {
	movq 208(%rsp), %rbx
	movq 88(%rsp), %r14
	testq %rbx, %rbx
	je .LBB226_335
		// crates/impact_voxel/src/object/extraction.rs:1174
		let mut neighbor_chunk_indices = chunk_indices;
	vmovaps 208(%rsp), %xmm0
	movq 224(%rsp), %rax
	movq %rax, 736(%rsp)
	vmovaps %xmm0, 720(%rsp)
		// crates/impact_voxel/src/object/extraction.rs:1175
		neighbor_chunk_indices[dim] -= 1;
	decq 720(%rsp)
	.cfi_escape 0x2e, 0x00
	movq 312(%rsp), %rdi
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hashbrown-0.16.1/src/set.rs:1093
		self.map.insert(value, ()).is_none()
	leaq 720(%rsp), %rsi
	callq <hashbrown::map::HashMap<[usize; 3], (), rustc_hash::FxBuildHasher>>::insert
.LBB226_335:
		// crates/impact_voxel/src/object/extraction.rs:1179
		if chunk_indices[dim] < self.chunk_counts[dim] - 1 {
	movq 184(%r14), %rax
	decq %rax
	cmpq %rax, %rbx
	jae .LBB226_337
		// crates/impact_voxel/src/object/extraction.rs:1180
		let mut neighbor_chunk_indices = chunk_indices;
	vmovaps 208(%rsp), %xmm0
	movq 224(%rsp), %rax
	movq %rax, 768(%rsp)
	vmovaps %xmm0, 752(%rsp)
		// crates/impact_voxel/src/object/extraction.rs:1181
		neighbor_chunk_indices[dim] += 1;
	incq 752(%rsp)
	.cfi_escape 0x2e, 0x00
	movq 312(%rsp), %rdi
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hashbrown-0.16.1/src/set.rs:1093
		self.map.insert(value, ()).is_none()
	leaq 752(%rsp), %rsi
	callq <hashbrown::map::HashMap<[usize; 3], (), rustc_hash::FxBuildHasher>>::insert
.LBB226_337:
		// crates/impact_voxel/src/object/extraction.rs:1173
		if chunk_indices[dim] > 0 {
	movq 216(%rsp), %rbx
	testq %rbx, %rbx
	je .LBB226_339
		// crates/impact_voxel/src/object/extraction.rs:1174
		let mut neighbor_chunk_indices = chunk_indices;
	vmovaps 208(%rsp), %xmm0
	movq 224(%rsp), %rax
	movq %rax, 736(%rsp)
	vmovaps %xmm0, 720(%rsp)
		// crates/impact_voxel/src/object/extraction.rs:1175
		neighbor_chunk_indices[dim] -= 1;
	decq 728(%rsp)
	.cfi_escape 0x2e, 0x00
	movq 312(%rsp), %rdi
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hashbrown-0.16.1/src/set.rs:1093
		self.map.insert(value, ()).is_none()
	leaq 720(%rsp), %rsi
	callq <hashbrown::map::HashMap<[usize; 3], (), rustc_hash::FxBuildHasher>>::insert
.LBB226_339:
		// crates/impact_voxel/src/object/extraction.rs:1179
		if chunk_indices[dim] < self.chunk_counts[dim] - 1 {
	movq 192(%r14), %rax
	decq %rax
	cmpq %rax, %rbx
	jae .LBB226_341
		// crates/impact_voxel/src/object/extraction.rs:1180
		let mut neighbor_chunk_indices = chunk_indices;
	vmovaps 208(%rsp), %xmm0
	movq 224(%rsp), %rax
	movq %rax, 768(%rsp)
	vmovaps %xmm0, 752(%rsp)
		// crates/impact_voxel/src/object/extraction.rs:1181
		neighbor_chunk_indices[dim] += 1;
	incq 760(%rsp)
	.cfi_escape 0x2e, 0x00
	movq 312(%rsp), %rdi
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hashbrown-0.16.1/src/set.rs:1093
		self.map.insert(value, ()).is_none()
	leaq 752(%rsp), %rsi
	callq <hashbrown::map::HashMap<[usize; 3], (), rustc_hash::FxBuildHasher>>::insert
.LBB226_341:
		// crates/impact_voxel/src/object/extraction.rs:1173
		if chunk_indices[dim] > 0 {
	movq 224(%rsp), %rbx
	testq %rbx, %rbx
	je .LBB226_343
		// crates/impact_voxel/src/object/extraction.rs:1174
		let mut neighbor_chunk_indices = chunk_indices;
	vmovaps 208(%rsp), %xmm0
	movq 224(%rsp), %rax
		// crates/impact_voxel/src/object/extraction.rs:1175
		neighbor_chunk_indices[dim] -= 1;
	decq %rax
		// crates/impact_voxel/src/object/extraction.rs:1174
		let mut neighbor_chunk_indices = chunk_indices;
	vmovaps %xmm0, 720(%rsp)
		// crates/impact_voxel/src/object/extraction.rs:1175
		neighbor_chunk_indices[dim] -= 1;
	movq %rax, 736(%rsp)
	.cfi_escape 0x2e, 0x00
	movq 312(%rsp), %rdi
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hashbrown-0.16.1/src/set.rs:1093
		self.map.insert(value, ()).is_none()
	leaq 720(%rsp), %rsi
	callq <hashbrown::map::HashMap<[usize; 3], (), rustc_hash::FxBuildHasher>>::insert
.LBB226_343:
		// crates/impact_voxel/src/object/extraction.rs:1179
		if chunk_indices[dim] < self.chunk_counts[dim] - 1 {
	movq 200(%r14), %rax
	decq %rax
	cmpq %rax, %rbx
	jae .LBB226_107
		// crates/impact_voxel/src/object/extraction.rs:1180
		let mut neighbor_chunk_indices = chunk_indices;
	vmovaps 208(%rsp), %xmm0
	movq 224(%rsp), %rax
		// crates/impact_voxel/src/object/extraction.rs:1181
		neighbor_chunk_indices[dim] += 1;
	incq %rax
		// crates/impact_voxel/src/object/extraction.rs:1180
		let mut neighbor_chunk_indices = chunk_indices;
	vmovaps %xmm0, 752(%rsp)
		// crates/impact_voxel/src/object/extraction.rs:1181
		neighbor_chunk_indices[dim] += 1;
	movq %rax, 768(%rsp)
	.cfi_escape 0x2e, 0x00
	movq 312(%rsp), %rdi
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hashbrown-0.16.1/src/set.rs:1093
		self.map.insert(value, ()).is_none()
	leaq 752(%rsp), %rsi
	callq <hashbrown::map::HashMap<[usize; 3], (), rustc_hash::FxBuildHasher>>::insert
	jmp .LBB226_107
.LBB226_345:
		// crates/impact_voxel/src/object/extraction.rs:1072
		self.chunks[chunk_idx] = VoxelChunk::Void;
	movb $3, 9(%rcx)
	jmp .LBB226_248
.LBB226_346:
	.cfi_escape 0x2e, 0x00
	movq 8(%rsp), %rax
	movq (%rsp), %r13
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/raw_vec/mod.rs:675
		do_reserve_and_handle(self, len, additional, elem_layout);
	movl $4096, %edx
	movl $1, %ecx
	movl $3, %r8d
	leaq 400(%rsp), %rdi
	movq %r12, %rsi
	movq %rax, 16(%rsp)
	vzeroupper
	callq <alloc::raw_vec::RawVecInner<_>>::reserve::do_reserve_and_handle::<alloc::alloc::Global>
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/vec/mod.rs:3054
		let len = self.len;
	movq 416(%rsp), %rbx
	movq 648(%rsp), %r12
	movq 568(%rsp), %rcx
	jmp .LBB226_159
.LBB226_348:
	.cfi_escape 0x2e, 0x00
	movq 8(%rsp), %rax
	movq (%rsp), %r13
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/raw_vec/mod.rs:675
		do_reserve_and_handle(self, len, additional, elem_layout);
	movl $4096, %edx
	movl $1, %ecx
	movl $3, %r8d
	leaq 400(%rsp), %rdi
	movq %r14, %rsi
	movq %rax, 16(%rsp)
	callq <alloc::raw_vec::RawVecInner<_>>::reserve::do_reserve_and_handle::<alloc::alloc::Global>
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/vec/mod.rs:3054
		let len = self.len;
	movq 416(%rsp), %r14
	jmp .LBB226_236
.LBB226_350:
	.cfi_escape 0x2e, 0x00
	movq 8(%rsp), %rax
	movq 1088(%rsp), %rdi
	movq (%rsp), %r13
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/raw_vec/mod.rs:675
		do_reserve_and_handle(self, len, additional, elem_layout);
	movl $4096, %edx
	movl $1, %ecx
	movl $3, %r8d
	movq %rbx, %rsi
	movq %rax, 16(%rsp)
	vzeroupper
	callq <alloc::raw_vec::RawVecInner<_>>::reserve::do_reserve_and_handle::<alloc::alloc::Global>
	movq 88(%rsp), %rdx
	movq 440(%rsp), %rcx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/vec/mod.rs:3054
		let len = self.len;
	movq 40(%rdx), %rax
	leaq 9(%rcx), %rdi
	jmp .LBB226_151
.LBB226_352:
	movq 1104(%rsp), %rax
	movq 304(%rsp), %rcx
	movq 1096(%rsp), %rdx
	movq 560(%rsp), %rbx
	movq %rax, 432(%rsp)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:1916
		fn lt(&self, other: &Self) -> bool { *self <  *other }
	cmpq 624(%rsp), %rax
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/iter/range.rs:900
		if self.start < self.end {
	jne .LBB226_103
	jmp .LBB226_354
.LBB226_353:
	movq 40(%rsp), %rax
	movq $0, 296(%rsp)
	movq %r13, 16(%rsp)
	xorl %ebx, %ebx
	xorl %ecx, %ecx
	xorl %ebp, %ebp
	movq %rax, 48(%rsp)
.LBB226_354:
	movq 816(%rsp), %rax
	movq 784(%rsp), %rdx
	movq 824(%rsp), %r8
		// crates/impact_voxel/src/object/extraction.rs:1191
		poly_split_detector_buffers,
	movq %rax, 1312(%rsp)
	movq %rdx, 1320(%rsp)
	movq 792(%rsp), %rdx
	movq $0, 1328(%rsp)
	movq %r8, 1336(%rsp)
	movq 832(%rsp), %r8
	movq %rdx, 1344(%rsp)
	movq 800(%rsp), %rdx
	movq $0, 1352(%rsp)
	movq %r8, 1360(%rsp)
	movq 840(%rsp), %r8
	movq %rdx, 1368(%rsp)
	movq 808(%rsp), %rdx
	movq $0, 1376(%rsp)
	movq %r8, 1384(%rsp)
	movq %rdx, 1392(%rsp)
	movq %rbp, %rdx
	xorl %ebp, %ebp
	movq $0, 1400(%rsp)
	.cfi_escape 0x2e, 0x00
	leaq 1696(%rsp), %rdi
	leaq 1312(%rsp), %rsi
	movq %rdx, 576(%rsp)
	movq %rcx, 304(%rsp)
		// crates/impact_voxel/src/object/extraction.rs:1190
		let mut poly_split_detector = SplitDetector::new(
	vzeroupper
	callq *<impact_voxel::object::split_detection::SplitDetector>::new@GOTPCREL(%rip)
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/allocator-api2-0.2.21/src/stable/vec/into_iter.rs:103
		if self.ptr == self.end {
	testq %rbx, %rbx
	je .LBB226_362
	movq 88(%rsp), %rax
	shlq $3, %rbx
	movq 104(%rsp), %r12
	leaq 1322(%rsp), %r15
	xorl %ebp, %ebp
	leaq (%rbx,%rbx,2), %r14
	movq 112(%rsp), %rbx
	addq $48, %rax
	movq %rax, 272(%rsp)
	jmp .LBB226_358
	.p2align	4
.LBB226_357:
	addq $24, %rbp
	cmpq %rbp, %r14
	je .LBB226_362
.LBB226_358:
	movq 32(%rsp), %rsi
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mod.rs:1733
		crate::intrinsics::read_via_copy(src)
	movzwl 14(%rsi,%rbp), %ecx
	movzbl 9(%rsi,%rbp), %eax
	movw %cx, 996(%rsp)
	movl 10(%rsi,%rbp), %ecx
	movl %ecx, 992(%rsp)
	movq 16(%rsi,%rbp), %rdx
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/allocator-api2-0.2.21/src/stable/vec/into_iter.rs:117
		Some(unsafe { ptr::read(old) })
	movq (%rsi,%rbp), %rcx
	movzbl 8(%rsi,%rbp), %esi
	movb %sil, 2040(%rsp)
	movq %rcx, 2032(%rsp)
		// crates/impact_voxel/src/object/extraction.rs:1196
		for (chunk, poly_chunk_idx) in non_uniform_chunks_inside {
	cmpb $-1, %al
	je .LBB226_362
	movzbl 2040(%rsp), %ecx
	movzwl 996(%rsp), %esi
	movb %cl, 1320(%rsp)
	movq 2032(%rsp), %rcx
	movq %rcx, 1312(%rsp)
	movl 992(%rsp), %ecx
	movb %al, 1321(%rsp)
	movw %si, 4(%r15)
	movl %ecx, (%r15)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/index.rs:278
		&mut (*slice)[self]
	cmpq %rbx, %rdx
	jae .LBB226_434
	movq %rdx, %rsi
	shlq $4, %rsi
		// crates/impact_voxel/src/object/extraction.rs:1200
		if let VoxelChunk::NonUniform(poly_chunk) = poly_chunk {
	cmpb $3, 9(%r12,%rsi)
	jae .LBB226_357
	addq %r12, %rsi
	.cfi_escape 0x2e, 0x00
	movq 272(%rsp), %rcx
		// crates/impact_voxel/src/object/extraction.rs:1201
		poly_split_detector.copy_local_connected_regions_from_chunk_in_other(
	leaq 1696(%rsp), %rdi
	leaq 1312(%rsp), %r8
	callq *<impact_voxel::object::split_detection::SplitDetector>::copy_local_connected_regions_from_chunk_in_other@GOTPCREL(%rip)
	jmp .LBB226_357
.LBB226_362:
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/allocator-api2-0.2.21/src/stable/raw_vec.rs:333
		if mem::size_of::<T>() == 0 || self.cap == 0 {
	testq %r13, %r13
	je .LBB226_364
		// crates/impact_alloc/src/arena.rs:169
		unsafe { &*self.arena_ptr }
	movq 72(%rsp), %rax
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:2505
		Bump::<MIN_ALIGN>::dealloc(self, ptr, layout)
	movq 16(%rax), %rax
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cell.rs:555
		unsafe { *self.value.get() }
	movq 32(%rax), %rcx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/non_null.rs:1714
		self.as_ptr() == other.as_ptr()
	cmpq 32(%rsp), %rcx
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:2231
		if self.is_last_allocation(ptr) {
	je .LBB226_370
.LBB226_364:
	movq 296(%rsp), %rax
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/allocator-api2-0.2.21/src/stable/vec/into_iter.rs:103
		if self.ptr == self.end {
	testq %rax, %rax
	je .LBB226_371
.LBB226_365:
	movq 128(%rsp), %r15
	imulq $56, %rax, %r14
	movq 408(%rsp), %rax
	movq 104(%rsp), %rbx
	movq 112(%rsp), %rbp
	movq 416(%rsp), %r12
	addq %r15, %r14
	movq %rax, 272(%rsp)
	jmp .LBB226_367
	.p2align	4
.LBB226_366:
	addq $56, %r15
	cmpq %r14, %r15
	je .LBB226_371
.LBB226_367:
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/allocator-api2-0.2.21/src/stable/vec/into_iter.rs:117
		Some(unsafe { ptr::read(old) })
	vmovdqu 8(%r15), %ymm0
	vmovdqu 40(%r15), %xmm1
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mod.rs:1733
		crate::intrinsics::read_via_copy(src)
	movq (%r15), %r8
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/allocator-api2-0.2.21/src/stable/vec/into_iter.rs:117
		Some(unsafe { ptr::read(old) })
	vmovdqa %xmm1, 1024(%rsp)
	vmovdqu %ymm0, 992(%rsp)
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/index.rs:278
		&mut (*slice)[self]
	cmpq %rbp, %r8
	jae .LBB226_428
	movq %r8, %rcx
	shlq $4, %rcx
		// crates/impact_voxel/src/object/extraction.rs:1212
		if let VoxelChunk::NonUniform(poly_chunk) = poly_chunk {
	cmpb $3, 9(%rbx,%rcx)
	jae .LBB226_366
	addq %rbx, %rcx
	.cfi_escape 0x2e, 0x00
	movq 272(%rsp), %rsi
		// crates/impact_voxel/src/object/extraction.rs:1214
		.update_local_connected_regions_within_occupied_ranges_for_chunk(
	leaq 1696(%rsp), %rdi
	leaq 992(%rsp), %r9
	movq %r12, %rdx
	vzeroupper
	callq *<impact_voxel::object::split_detection::SplitDetector>::update_local_connected_regions_within_occupied_ranges_for_chunk@GOTPCREL(%rip)
	jmp .LBB226_366
.LBB226_370:
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/alloc/layout.rs:573
		if element_size != 0 && n > Layout::max_size_for_alignment(alignment) / element_size {
	leaq (%r13,%r13,2), %rdx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mut_ptr.rs:961
		unsafe { intrinsics::offset(self, count) }
	leaq (%rcx,%rdx,8), %rcx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/mem/mod.rs:931
		crate::intrinsics::write_via_move(dest, src);
	movq %rcx, 32(%rax)
	movq 296(%rsp), %rax
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/allocator-api2-0.2.21/src/stable/vec/into_iter.rs:103
		if self.ptr == self.end {
	testq %rax, %rax
	jne .LBB226_365
.LBB226_371:
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/allocator-api2-0.2.21/src/stable/raw_vec.rs:333
		if mem::size_of::<T>() == 0 || self.cap == 0 {
	cmpq $0, 16(%rsp)
	je .LBB226_374
		// crates/impact_alloc/src/arena.rs:169
		unsafe { &*self.arena_ptr }
	movq 72(%rsp), %rax
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
	jne .LBB226_374
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/alloc/layout.rs:573
		if element_size != 0 && n > Layout::max_size_for_alignment(alignment) / element_size {
	imulq $56, 16(%rsp), %rdx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mut_ptr.rs:961
		unsafe { intrinsics::offset(self, count) }
	addq %rdx, %rcx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/mem/mod.rs:931
		crate::intrinsics::write_via_move(dest, src);
	movq %rcx, 32(%rax)
.LBB226_374:
	.cfi_escape 0x2e, 0x00
	movq 88(%rsp), %r14
		// crates/impact_voxel/src/object/extraction.rs:1225
		self.update_occupied_ranges();
	movq %r14, %rdi
	vzeroupper
	callq *<impact_voxel::object::VoxelObject>::update_occupied_ranges@GOTPCREL(%rip)
	movq 608(%rsp), %rbx
	movq 576(%rsp), %r15
	.cfi_escape 0x2e, 0x00
	leaq 1040(%rsp), %rsi
		// crates/impact_voxel/src/object/extraction.rs:1227
		self.update_upper_boundary_adjacencies_along_dim_for_chunks(invalidated_upper_face_chunks);
	movq %r14, %rdi
	callq <impact_voxel::object::VoxelObject>::update_upper_boundary_adjacencies_along_dim_for_chunks::<hashbrown::set::HashSet<(usize, impact_voxel::utils::Dimension), rustc_hash::FxBuildHasher, &impact_alloc::arena::PoolArena>>
	.cfi_escape 0x2e, 0x00
		// crates/impact_voxel/src/object/extraction.rs:1231
		self.resolve_connected_regions_between_all_chunks();
	movq %r14, %rdi
	callq *<impact_voxel::object::VoxelObject>::resolve_connected_regions_between_all_chunks@GOTPCREL(%rip)
	vmovdqu 1280(%rsp), %ymm2
	movq 776(%rsp), %rax
	vmovdqa 1808(%rsp), %xmm4
		// crates/impact_voxel/src/object/extraction.rs:1233
		let extraction_result = Self::complete_extracted_voxel_object(
	vmovdqa .LCPI226_20(%rip), %ymm3
		// crates/impact_voxel/src/object/extraction.rs:1243
		poly_invalidated_mesh_chunk_indices,
	vmovaps 1232(%rsp), %xmm1
		// crates/impact_voxel/src/object/extraction.rs:1234
		self.voxel_extent,
	vmovss 352(%r14), %xmm0
	movq 616(%rsp), %r12
		// crates/impact_voxel/src/object/extraction.rs:1235
		&self.origin_offset_in_root,
	addq $328, %r14
		// crates/impact_voxel/src/object/extraction.rs:1243
		poly_invalidated_mesh_chunk_indices,
	movq %rax, 2000(%rsp)
	movq 848(%rsp), %rax
	movq %r12, 2008(%rsp)
		// crates/impact_voxel/src/object/extraction.rs:1233
		let extraction_result = Self::complete_extracted_voxel_object(
	vpermi2q 1248(%rsp), %ymm2, %ymm3
	vmovdqa %xmm4, 1968(%rsp)
		// crates/impact_voxel/src/object/extraction.rs:1243
		poly_invalidated_mesh_chunk_indices,
	vmovups %xmm1, 2016(%rsp)
		// crates/impact_voxel/src/object/extraction.rs:1233
		let extraction_result = Self::complete_extracted_voxel_object(
	movq %rax, 1984(%rsp)
	movq 328(%rsp), %rax
	vmovdqu %ymm3, 1312(%rsp)
	movq %rax, 1344(%rsp)
	movq 336(%rsp), %rax
	movq %rax, 1352(%rsp)
	.cfi_escape 0x2e, 0x20
	movq 304(%rsp), %r9
	leaq 2000(%rsp), %rax
	leaq 1968(%rsp), %rdx
	leaq 96(%rsp), %r10
	leaq 400(%rsp), %r11
	leaq 2032(%rsp), %rdi
	leaq 1312(%rsp), %rcx
	movq %r14, %rsi
	movq %r15, %r8
	pushq %rax
	.cfi_adjust_cfa_offset 8
	leaq 1704(%rsp), %rax
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
		// crates/impact_voxel/src/object/extraction.rs:1246
		let mut extracted = match extraction_result {
	cmpq $-1, 2032(%rsp)
	je .LBB226_393
		// crates/impact_voxel/src/object/extraction.rs:1247
		ExtractionResult::Extracted(extracted) => extracted,
	vmovups 2288(%rsp), %zmm1
	vmovups 2352(%rsp), %zmm0
	vmovdqu64 2032(%rsp), %zmm4
	vmovups 2160(%rsp), %zmm2
	vmovdqu64 2224(%rsp), %zmm3
	vmovups %zmm1, 1568(%rsp)
	vmovups 2096(%rsp), %zmm1
	vmovups %zmm0, 1632(%rsp)
	vmovdqu64 %zmm3, 1504(%rsp)
	vmovups %zmm2, 1440(%rsp)
	vmovdqu64 %zmm4, 1312(%rsp)
	vmovups %zmm1, 1376(%rsp)
	.cfi_escape 0x2e, 0x00
	leaq 1312(%rsp), %rdi
		// crates/impact_voxel/src/object/extraction.rs:1259
		.update_all_chunk_boundary_adjacencies();
	vzeroupper
	callq *<impact_voxel::object::VoxelObject>::update_all_chunk_boundary_adjacencies@GOTPCREL(%rip)
	.cfi_escape 0x2e, 0x00
	leaq 1312(%rsp), %rdi
		// crates/impact_voxel/src/object/extraction.rs:1263
		.resolve_connected_regions_between_all_chunks();
	callq *<impact_voxel::object::VoxelObject>::resolve_connected_regions_between_all_chunks@GOTPCREL(%rip)
	.cfi_escape 0x2e, 0x00
	leaq 1312(%rsp), %rdi
		// crates/impact_voxel/src/object/extraction.rs:1266
		extracted.voxel_object.update_occupied_ranges();
	callq *<impact_voxel::object::VoxelObject>::update_occupied_ranges@GOTPCREL(%rip)
		// crates/impact_voxel/src/object/extraction.rs:1268
		ExtractionResult::Extracted(extracted)
	vmovups 1568(%rsp), %zmm1
	vmovdqu64 1632(%rsp), %zmm0
	vmovdqu64 1312(%rsp), %zmm4
	vmovdqu64 1440(%rsp), %zmm2
	vmovdqu64 1504(%rsp), %zmm3
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/allocator-api2-0.2.21/src/stable/raw_vec.rs:333
		if mem::size_of::<T>() == 0 || self.cap == 0 {
	cmpq $0, 48(%rsp)
		// crates/impact_voxel/src/object/extraction.rs:1268
		ExtractionResult::Extracted(extracted)
	vmovups %zmm1, 256(%rbx)
	vmovdqu64 1376(%rsp), %zmm1
	vmovdqu64 %zmm0, 320(%rbx)
	vmovdqu64 %zmm3, 192(%rbx)
	vmovdqu64 %zmm2, 128(%rbx)
	vmovdqu64 %zmm4, (%rbx)
	vmovdqu64 %zmm1, 64(%rbx)
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/allocator-api2-0.2.21/src/stable/raw_vec.rs:333
		if mem::size_of::<T>() == 0 || self.cap == 0 {
	je .LBB226_385
		// crates/impact_alloc/src/arena.rs:169
		unsafe { &*self.arena_ptr }
	movq 72(%rsp), %rax
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:2505
		Bump::<MIN_ALIGN>::dealloc(self, ptr, layout)
	movq 16(%rax), %rax
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cell.rs:555
		unsafe { *self.value.get() }
	movq 32(%rax), %rcx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/non_null.rs:1714
		self.as_ptr() == other.as_ptr()
	cmpq 192(%rsp), %rcx
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:2231
		if self.is_last_allocation(ptr) {
	jne .LBB226_385
	movq 48(%rsp), %rdx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/alloc/layout.rs:573
		if element_size != 0 && n > Layout::max_size_for_alignment(alignment) / element_size {
	shlq $4, %rdx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mut_ptr.rs:961
		unsafe { intrinsics::offset(self, count) }
	addq %rdx, %rcx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/mem/mod.rs:931
		crate::intrinsics::write_via_move(dest, src);
	movq %rcx, 32(%rax)
.LBB226_385:
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/allocator-api2-0.2.21/src/stable/raw_vec.rs:333
		if mem::size_of::<T>() == 0 || self.cap == 0 {
	cmpq $0, 136(%rsp)
	je .LBB226_388
		// crates/impact_alloc/src/arena.rs:169
		unsafe { &*self.arena_ptr }
	movq 72(%rsp), %rax
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:2505
		Bump::<MIN_ALIGN>::dealloc(self, ptr, layout)
	movq 16(%rax), %rax
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cell.rs:555
		unsafe { *self.value.get() }
	movq 32(%rax), %rcx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/non_null.rs:1714
		self.as_ptr() == other.as_ptr()
	cmpq 184(%rsp), %rcx
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:2231
		if self.is_last_allocation(ptr) {
	jne .LBB226_388
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
.LBB226_388:
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/allocator-api2-0.2.21/src/stable/raw_vec.rs:333
		if mem::size_of::<T>() == 0 || self.cap == 0 {
	cmpq $0, 144(%rsp)
	je .LBB226_391
		// crates/impact_alloc/src/arena.rs:169
		unsafe { &*self.arena_ptr }
	movq 72(%rsp), %rax
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
	jne .LBB226_391
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
.LBB226_391:
		// crates/impact_voxel/src/object/extraction.rs:1269
		}
	movq 80(%rsp), %rdi
	xorl %ebp, %ebp
	.cfi_escape 0x2e, 0x00
	vzeroupper
	callq core::ptr::drop_glue::<impact_alloc::arena::PoolArena>
	jmp .LBB226_403
.LBB226_393:
		// crates/impact_voxel/src/object/extraction.rs:1248
		result @ ExtractionResult::NotExtracted(_) => {
	vmovups 2288(%rsp), %zmm1
	vmovdqu64 2352(%rsp), %zmm0
	vmovdqu64 2032(%rsp), %zmm4
	vmovdqu64 2160(%rsp), %zmm2
	vmovdqu64 2224(%rsp), %zmm3
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/allocator-api2-0.2.21/src/stable/raw_vec.rs:333
		if mem::size_of::<T>() == 0 || self.cap == 0 {
	cmpq $0, 48(%rsp)
		// crates/impact_voxel/src/object/extraction.rs:1248
		result @ ExtractionResult::NotExtracted(_) => {
	vmovups %zmm1, 256(%rbx)
	vmovdqu64 2096(%rsp), %zmm1
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
	movq 72(%rsp), %rax
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:2505
		Bump::<MIN_ALIGN>::dealloc(self, ptr, layout)
	movq 16(%rax), %rax
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cell.rs:555
		unsafe { *self.value.get() }
	movq 32(%rax), %rcx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/non_null.rs:1714
		self.as_ptr() == other.as_ptr()
	cmpq 192(%rsp), %rcx
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:2231
		if self.is_last_allocation(ptr) {
	jne .LBB226_396
	movq 48(%rsp), %rdx
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
	movq 72(%rsp), %rax
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:2505
		Bump::<MIN_ALIGN>::dealloc(self, ptr, layout)
	movq 16(%rax), %rax
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cell.rs:555
		unsafe { *self.value.get() }
	movq 32(%rax), %rcx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/non_null.rs:1714
		self.as_ptr() == other.as_ptr()
	cmpq 184(%rsp), %rcx
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
	movq 72(%rsp), %rax
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
		// crates/impact_voxel/src/object/extraction.rs:1269
		}
	movq 80(%rsp), %rdi
	xorl %ebp, %ebp
	.cfi_escape 0x2e, 0x00
	vzeroupper
	callq core::ptr::drop_glue::<impact_alloc::arena::PoolArena>
.LBB226_403:
	movq 608(%rsp), %rdi
.LBB226_404:
		// crates/impact_voxel/src/object/extraction.rs:631
		}
	movq %rdi, %rax
	addq $2424, %rsp
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
.LBB226_405:
	.cfi_def_cfa_offset 2480
	movq 248(%rsp), %rsi
	movq %rbp, %rdi
	leaq .Lanon.5bbf1cbe9e9f837e6488bd4d315a6b9f.641(%rip), %rdx
.LBB226_406:
	movb $1, %bpl
	.cfi_escape 0x2e, 0x00
	movq 8(%rsp), %rax
	movq (%rsp), %r13
	movq %rax, 16(%rsp)
	vzeroupper
	callq *core::panicking::panic_bounds_check@GOTPCREL(%rip)
	jmp .LBB226_441
.LBB226_407:
	movb $1, %bpl
	.cfi_escape 0x2e, 0x00
	movq 8(%rsp), %rax
	movq (%rsp), %r13
	movq %rax, 16(%rsp)
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/allocator-api2-0.2.21/src/stable/raw_vec.rs:612
		Err(CapacityOverflow) => capacity_overflow(),
	vzeroupper
	callq *allocator_api2::stable::raw_vec::capacity_overflow@GOTPCREL(%rip)
	jmp .LBB226_441
.LBB226_408:
	movb $1, %bpl
	.cfi_escape 0x2e, 0x00
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/index.rs:272
		&(*slice)[self]
	leaq .Lanon.5bbf1cbe9e9f837e6488bd4d315a6b9f.35(%rip), %rdx
	vzeroupper
	callq *core::panicking::panic_bounds_check@GOTPCREL(%rip)
	jmp .LBB226_441
.LBB226_409:
	movb $1, %bpl
	.cfi_escape 0x2e, 0x00
	movq 8(%rsp), %rax
	movq (%rsp), %r13
		// crates/impact_voxel/src/object/extraction.rs:859
		unreachable!();
	leaq .Lanon.5bbf1cbe9e9f837e6488bd4d315a6b9f.16(%rip), %rdi
	leaq .Lanon.5bbf1cbe9e9f837e6488bd4d315a6b9f.31(%rip), %rdx
	movl $40, %esi
	movq %rax, 16(%rsp)
	vzeroupper
	callq *core::panicking::panic@GOTPCREL(%rip)
	jmp .LBB226_441
.LBB226_410:
	movq 640(%rsp), %rdx
	leaq .Lanon.5bbf1cbe9e9f837e6488bd4d315a6b9f.44(%rip), %rcx
	jmp .LBB226_412
.LBB226_411:
	movq %r12, %r15
	movq %rbx, %rdx
	leaq .Lanon.5bbf1cbe9e9f837e6488bd4d315a6b9f.30(%rip), %rcx
.LBB226_412:
	movb $1, %bpl
	.cfi_escape 0x2e, 0x00
	movq 8(%rsp), %rax
	movq (%rsp), %r13
	movq %r15, %rdi
	movq %rbx, %rsi
	movq %rax, 16(%rsp)
	vzeroupper
	callq *core::slice::index::slice_index_fail@GOTPCREL(%rip)
	jmp .LBB226_441
.LBB226_413:
	movb $1, %bpl
	.cfi_escape 0x2e, 0x00
	movq 8(%rsp), %rax
	movq (%rsp), %r13
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/index.rs:456
		slice_index_fail(self.start, self.end, slice.len())
	leaq .Lanon.5bbf1cbe9e9f837e6488bd4d315a6b9f.44(%rip), %rcx
	movq %rax, 16(%rsp)
	vzeroupper
	callq *core::slice::index::slice_index_fail@GOTPCREL(%rip)
	jmp .LBB226_441
.LBB226_414:
	movb $1, %bpl
	.cfi_escape 0x2e, 0x00
	movq 8(%rsp), %rax
	movq %rax, 16(%rsp)
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/allocator-api2-0.2.21/src/stable/raw_vec.rs:612
		Err(CapacityOverflow) => capacity_overflow(),
	callq *allocator_api2::stable::raw_vec::capacity_overflow@GOTPCREL(%rip)
	jmp .LBB226_441
.LBB226_415:
	movb $1, %bpl
	.cfi_escape 0x2e, 0x00
	movq (%rsp), %r13
	movq %rax, 16(%rsp)
	callq *allocator_api2::stable::raw_vec::capacity_overflow@GOTPCREL(%rip)
	jmp .LBB226_441
.LBB226_416:
	.cfi_escape 0x2e, 0x00
	leaq 400(%rsp), %rdi
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/raw_vec/mod.rs:675
		do_reserve_and_handle(self, len, additional, elem_layout);
	movl $1, %ecx
	movl $3, %r8d
	callq <alloc::raw_vec::RawVecInner<_>>::reserve::do_reserve_and_handle::<alloc::alloc::Global>
	jmp .LBB226_83
.LBB226_417:
	.cfi_escape 0x2e, 0x00
	leaq 96(%rsp), %rdi
	movl $4, %ecx
	movl $16, %r8d
	vzeroupper
	callq <alloc::raw_vec::RawVecInner<_>>::reserve::do_reserve_and_handle::<alloc::alloc::Global>
	jmp .LBB226_84
.LBB226_418:
	leaq .Lanon.5bbf1cbe9e9f837e6488bd4d315a6b9f.32(%rip), %rdx
	jmp .LBB226_420
.LBB226_419:
	leaq .Lanon.5bbf1cbe9e9f837e6488bd4d315a6b9f.33(%rip), %rdx
.LBB226_420:
	.cfi_escape 0x2e, 0x00
	movq 488(%rsp), %rdi
	movl $3, %esi
	callq *core::panicking::panic_bounds_check@GOTPCREL(%rip)
	jmp .LBB226_441
.LBB226_421:
	.cfi_escape 0x2e, 0x00
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/allocator-api2-0.2.21/src/stable/raw_vec.rs:272
		Err(_) => handle_alloc_error(layout),
	movl $16, %edi
	movq %rbx, %rsi
	callq *alloc::alloc::handle_alloc_error@GOTPCREL(%rip)
	jmp .LBB226_441
.LBB226_422:
	.cfi_escape 0x2e, 0x00
	movl $16, %edi
	movq %rbx, %rsi
	callq *alloc::alloc::handle_alloc_error@GOTPCREL(%rip)
	jmp .LBB226_441
.LBB226_423:
	.cfi_escape 0x2e, 0x00
	movl $8, %edi
	movq %rbx, %rsi
	callq *alloc::alloc::handle_alloc_error@GOTPCREL(%rip)
	jmp .LBB226_441
.LBB226_424:
	.cfi_escape 0x2e, 0x00
	movl $8, %edi
	movq %rbx, %rsi
	callq *alloc::alloc::handle_alloc_error@GOTPCREL(%rip)
	jmp .LBB226_441
.LBB226_425:
	movb $1, %bpl
	.cfi_escape 0x2e, 0x00
	movq 8(%rsp), %rax
	movq 40(%rsp), %rsi
	movq (%rsp), %r13
		// crates/impact_voxel/src/object/extraction.rs:768
		intersecting_planes.push(normalized_face_planes[plane_idx]);
	leaq .Lanon.5bbf1cbe9e9f837e6488bd4d315a6b9f.34(%rip), %rdx
	movq %rbx, %rdi
	movq %rax, 16(%rsp)
	vzeroupper
	callq *core::panicking::panic_bounds_check@GOTPCREL(%rip)
	jmp .LBB226_441
.LBB226_426:
	movb $1, %bpl
	.cfi_escape 0x2e, 0x00
	movq 8(%rsp), %rax
	movq (%rsp), %r13
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/index.rs:272
		&(*slice)[self]
	leaq .Lanon.5bbf1cbe9e9f837e6488bd4d315a6b9f.28(%rip), %rdx
	movq %rax, 16(%rsp)
	vzeroupper
	callq *core::panicking::panic_bounds_check@GOTPCREL(%rip)
	jmp .LBB226_441
.LBB226_427:
	movb $1, %bpl
	.cfi_escape 0x2e, 0x00
	movq 8(%rsp), %rax
	movq (%rsp), %r13
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/allocator-api2-0.2.21/src/stable/raw_vec.rs:613
		Err(AllocError { layout, .. }) => handle_alloc_error(layout),
	movl $4, %edi
	movq %rdx, %rsi
	movq %rax, 16(%rsp)
	vzeroupper
	callq *alloc::alloc::handle_alloc_error@GOTPCREL(%rip)
	jmp .LBB226_441
.LBB226_428:
	.cfi_escape 0x2e, 0x00
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/index.rs:278
		&mut (*slice)[self]
	leaq .Lanon.5bbf1cbe9e9f837e6488bd4d315a6b9f.26(%rip), %rdx
	movq %r8, %rdi
	movq %rbp, %rsi
	vzeroupper
	callq *core::panicking::panic_bounds_check@GOTPCREL(%rip)
	jmp .LBB226_441
.LBB226_429:
	movq 320(%rsp), %rdi
	leaq .Lanon.5bbf1cbe9e9f837e6488bd4d315a6b9f.29(%rip), %rdx
	jmp .LBB226_406
.LBB226_430:
	.cfi_escape 0x2e, 0x00
	movq 248(%rsp), %rsi
		// crates/impact_voxel/src/object/inertia.rs:709
		let density = voxel_type_densities[voxel_type.idx()];
	leaq .Lanon.5bbf1cbe9e9f837e6488bd4d315a6b9f.642(%rip), %rdx
	vzeroupper
	callq *core::panicking::panic_bounds_check@GOTPCREL(%rip)
	jmp .LBB226_441
.LBB226_431:
	.cfi_escape 0x2e, 0x00
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/allocator-api2-0.2.21/src/stable/raw_vec.rs:612
		Err(CapacityOverflow) => capacity_overflow(),
	callq *allocator_api2::stable::raw_vec::capacity_overflow@GOTPCREL(%rip)
	jmp .LBB226_441
.LBB226_432:
	.cfi_escape 0x2e, 0x00
	callq *allocator_api2::stable::raw_vec::capacity_overflow@GOTPCREL(%rip)
	jmp .LBB226_441
.LBB226_433:
	.cfi_escape 0x2e, 0x00
	movq 272(%rsp), %rax
	movq 176(%rsp), %rcx
	movq 144(%rsp), %r12
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/allocator-api2-0.2.21/src/stable/raw_vec.rs:613
		Err(AllocError { layout, .. }) => handle_alloc_error(layout),
	movl $16, %edi
	movq %rbx, %rsi
	movq %rax, 136(%rsp)
	movq %rcx, 160(%rsp)
	callq *alloc::alloc::handle_alloc_error@GOTPCREL(%rip)
	jmp .LBB226_441
.LBB226_434:
	.cfi_escape 0x2e, 0x00
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/index.rs:278
		&mut (*slice)[self]
	leaq .Lanon.5bbf1cbe9e9f837e6488bd4d315a6b9f.27(%rip), %rax
	movq %rdx, %rdi
	movq %rbx, %rsi
	movq %rax, %rdx
	callq *core::panicking::panic_bounds_check@GOTPCREL(%rip)
	jmp .LBB226_441
.LBB226_435:
	.cfi_escape 0x2e, 0x00
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/allocator-api2-0.2.21/src/stable/raw_vec.rs:613
		Err(AllocError { layout, .. }) => handle_alloc_error(layout),
	movl $16, %edi
	movq %rdx, %rsi
	callq *alloc::alloc::handle_alloc_error@GOTPCREL(%rip)
	jmp .LBB226_441
.LBB226_436:
	movb $1, %bpl
	.cfi_escape 0x2e, 0x00
	movq 8(%rsp), %rax
	movq (%rsp), %r13
	movl $8, %edi
	movq %rdx, %rsi
	movq %rax, 16(%rsp)
	callq *alloc::alloc::handle_alloc_error@GOTPCREL(%rip)
	jmp .LBB226_441
.LBB226_437:
	movb $1, %bpl
	.cfi_escape 0x2e, 0x00
	movq 8(%rsp), %rax
	movq (%rsp), %r13
	movl $8, %edi
	movq %rdx, %rsi
	movq %rax, 16(%rsp)
	callq *alloc::alloc::handle_alloc_error@GOTPCREL(%rip)
	jmp .LBB226_441
.LBB226_438:
	.cfi_escape 0x2e, 0x00
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/allocator-api2-0.2.21/src/stable/raw_vec.rs:260
		Err(_) => capacity_overflow(),
	callq *allocator_api2::stable::raw_vec::capacity_overflow@GOTPCREL(%rip)
	jmp .LBB226_441
.LBB226_439:
	.cfi_escape 0x2e, 0x00
	vzeroupper
	callq *allocator_api2::stable::raw_vec::capacity_overflow@GOTPCREL(%rip)
	jmp .LBB226_441
.LBB226_440:
	.cfi_escape 0x2e, 0x00
	vzeroupper
	callq *allocator_api2::stable::raw_vec::capacity_overflow@GOTPCREL(%rip)
.LBB226_441:
	ud2
	movq %rax, %r15
	jmp .LBB226_511
	movq %rax, %r15
	.cfi_escape 0x2e, 0x00
	leaq 1696(%rsp), %rdi
		// crates/impact_voxel/src/object/extraction.rs:1269
		}
	callq core::ptr::drop_glue::<impact_voxel::object::split_detection::SplitDetector>
	xorl %ebp, %ebp
	xorl %eax, %eax
	xorl %r12d, %r12d
	jmp .LBB226_484
	movq 40(%rsp), %rcx
	movq %rax, %r15
	movb $1, %bpl
	movb $1, %al
	movq %r13, 16(%rsp)
	movq %rcx, 48(%rsp)
	jmp .LBB226_488
	movq %rax, %r15
	.cfi_escape 0x2e, 0x00
	leaq 1312(%rsp), %rdi
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mod.rs:825
		pub(crate) const unsafe fn drop_glue<T: PointeeSized>(_: &mut T)
	callq core::ptr::drop_glue::<impact_voxel::object::VoxelObject>
	jmp .LBB226_511
	movq %rax, %r15
	.cfi_escape 0x2e, 0x00
	leaq 1696(%rsp), %rdi
		// crates/impact_voxel/src/object/extraction.rs:1269
		}
	callq core::ptr::drop_glue::<impact_voxel::object::split_detection::SplitDetector>
	movq 616(%rsp), %rsi
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hashbrown-0.16.1/src/raw/mod.rs:2312
		if !self.is_empty_singleton() {
	testq %rsi, %rsi
	jne .LBB226_505
	jmp .LBB226_507
	jmp .LBB226_451
	jmp .LBB226_456
	movq 584(%rsp), %rcx
	movq %rax, %r15
	movb $1, %bpl
	movq %rcx, 144(%rsp)
	jmp .LBB226_465
.LBB226_451:
	movb $1, %r12b
	movq %rax, %r15
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/allocator-api2-0.2.21/src/stable/raw_vec.rs:333
		if mem::size_of::<T>() == 0 || self.cap == 0 {
	testq %r13, %r13
	je .LBB226_463
		// crates/impact_alloc/src/arena.rs:169
		unsafe { &*self.arena_ptr }
	movq 72(%rsp), %rax
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:2505
		Bump::<MIN_ALIGN>::dealloc(self, ptr, layout)
	movq 16(%rax), %rax
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cell.rs:555
		unsafe { *self.value.get() }
	movq 32(%rax), %rcx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/non_null.rs:1714
		self.as_ptr() == other.as_ptr()
	cmpq 32(%rsp), %rcx
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:2231
		if self.is_last_allocation(ptr) {
	jne .LBB226_463
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/alloc/layout.rs:573
		if element_size != 0 && n > Layout::max_size_for_alignment(alignment) / element_size {
	leaq (%r13,%r13,2), %rdx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mut_ptr.rs:961
		unsafe { intrinsics::offset(self, count) }
	leaq (%rcx,%rdx,8), %rcx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/mem/mod.rs:931
		crate::intrinsics::write_via_move(dest, src);
	movq %rcx, 32(%rax)
	jmp .LBB226_463
	jmp .LBB226_459
.LBB226_456:
	movq 272(%rsp), %rcx
	movq %rax, %r15
	movq 40(%rsp), %rax
	movb $1, %bpl
	movq %rcx, 136(%rsp)
	jmp .LBB226_512
	jmp .LBB226_480
.LBB226_459:
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/allocator-api2-0.2.21/src/stable/raw_vec.rs:333
		if mem::size_of::<T>() == 0 || self.cap == 0 {
	cmpq $0, 16(%rsp)
	movq %rax, %r15
	je .LBB226_462
		// crates/impact_alloc/src/arena.rs:169
		unsafe { &*self.arena_ptr }
	movq 72(%rsp), %rax
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
	jne .LBB226_462
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/alloc/layout.rs:573
		if element_size != 0 && n > Layout::max_size_for_alignment(alignment) / element_size {
	imulq $56, 16(%rsp), %rdx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mut_ptr.rs:961
		unsafe { intrinsics::offset(self, count) }
	addq %rdx, %rcx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/mem/mod.rs:931
		crate::intrinsics::write_via_move(dest, src);
	movq %rcx, 32(%rax)
.LBB226_462:
	xorl %r12d, %r12d
.LBB226_463:
	.cfi_escape 0x2e, 0x00
	leaq 1696(%rsp), %rdi
		// crates/impact_voxel/src/object/extraction.rs:1269
		}
	callq core::ptr::drop_glue::<impact_voxel::object::split_detection::SplitDetector>
	xorl %ebp, %ebp
	xorl %eax, %eax
	jmp .LBB226_484
	movb $1, %bpl
	movq %rax, %r15
	movq %r12, 144(%rsp)
.LBB226_465:
	movq 160(%rsp), %rax
	movq %rax, 176(%rsp)
	movq 40(%rsp), %rax
	jmp .LBB226_512
	jmp .LBB226_480
	jmp .LBB226_480
	jmp .LBB226_480
	jmp .LBB226_471
.LBB226_471:
	movq %rax, %r15
	movb $1, %bpl
	jmp .LBB226_483
	movq 40(%rsp), %rcx
	movq %rax, %r15
	movb $1, %bpl
	movq %rcx, 144(%rsp)
	movq %rcx, 136(%rsp)
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/allocator-api2-0.2.21/src/stable/raw_vec.rs:333
		if mem::size_of::<T>() == 0 || self.cap == 0 {
	jmp .LBB226_516
	movq %rax, %r15
	movb $1, %bpl
	jmp .LBB226_519
	movq %rax, %r15
	movb $1, %bpl
	jmp .LBB226_496
	movq %rax, %r15
		// crates/impact_voxel/src/object/extraction.rs:1269
		}
	jmp .LBB226_499
	movb $1, %bpl
	movq %rax, %r15
	jmp .LBB226_521
	movq %rax, %r15
	movb $1, %bpl
	movb $1, %al
	movb $1, %r12b
	movq %rbx, 32(%rsp)
	movq %r14, %r13
	jmp .LBB226_481
	movq %rax, %r15
	jmp .LBB226_522
.LBB226_480:
	movq (%rsp), %r13
	movq %rax, %r15
	movb $1, %bpl
	movb $1, %al
	movb $1, %r12b
.LBB226_481:
	movq 8(%rsp), %rcx
	movq %rcx, 16(%rsp)
	jmp .LBB226_484
	movq %rax, %r15
.LBB226_483:
	movb $1, %al
	movb $1, %r12b
.LBB226_484:
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hashbrown-0.16.1/src/raw/mod.rs:3520
		.drop_inner_table::<T, _>(&self.alloc, Self::TABLE_LAYOUT);
	movq 1048(%rsp), %rcx
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hashbrown-0.16.1/src/raw/mod.rs:2312
		if !self.is_empty_singleton() {
	testq %rcx, %rcx
	je .LBB226_487
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hashbrown-0.16.1/src/raw/mod.rs:3520
		.drop_inner_table::<T, _>(&self.alloc, Self::TABLE_LAYOUT);
	movq 1072(%rsp), %rsi
	movq 1040(%rsp), %r8
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
	jne .LBB226_487
	leaq 33(%rdx,%rcx), %rcx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mut_ptr.rs:961
		unsafe { intrinsics::offset(self, count) }
	addq %rcx, %rdi
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/mem/mod.rs:931
		crate::intrinsics::write_via_move(dest, src);
	movq %rdi, 32(%rsi)
.LBB226_487:
		// crates/impact_voxel/src/object/extraction.rs:1269
		}
	testb %r12b, %r12b
	je .LBB226_491
.LBB226_488:
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/allocator-api2-0.2.21/src/stable/raw_vec.rs:333
		if mem::size_of::<T>() == 0 || self.cap == 0 {
	cmpq $0, 16(%rsp)
	je .LBB226_491
		// crates/impact_alloc/src/arena.rs:169
		unsafe { &*self.arena_ptr }
	movq 72(%rsp), %rcx
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
	jne .LBB226_491
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/alloc/layout.rs:573
		if element_size != 0 && n > Layout::max_size_for_alignment(alignment) / element_size {
	imulq $56, 16(%rsp), %rsi
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mut_ptr.rs:961
		unsafe { intrinsics::offset(self, count) }
	addq %rsi, %rdx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/mem/mod.rs:931
		crate::intrinsics::write_via_move(dest, src);
	movq %rdx, 32(%rcx)
.LBB226_491:
		// crates/impact_voxel/src/object/extraction.rs:1269
		}
	testb %al, %al
	je .LBB226_495
	movq 48(%rsp), %rax
	movq %rax, 40(%rsp)
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/allocator-api2-0.2.21/src/stable/raw_vec.rs:333
		if mem::size_of::<T>() == 0 || self.cap == 0 {
	testq %r13, %r13
	jne .LBB226_496
		// crates/impact_voxel/src/object/extraction.rs:1269
		}
	testb %bpl, %bpl
	je .LBB226_494
.LBB226_499:
	movq 816(%rsp), %rsi
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/raw_vec/mod.rs:634
		if elem_layout.size() == 0 || self.cap.as_inner() == 0 {
	testq %rsi, %rsi
	jne .LBB226_526
	movq 824(%rsp), %rsi
	testq %rsi, %rsi
	jne .LBB226_527
.LBB226_501:
	movq 832(%rsp), %rsi
	testq %rsi, %rsi
	jne .LBB226_528
.LBB226_502:
	movq 840(%rsp), %rsi
	testq %rsi, %rsi
	je .LBB226_504
.LBB226_503:
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/num/uint_macros.rs:1359
		intrinsics::unchecked_mul(self, rhs)
	addq %rsi, %rsi
	.cfi_escape 0x2e, 0x00
	movq 808(%rsp), %rdi
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/alloc.rs:128
		unsafe { __rust_dealloc(ptr, layout.size(), layout.alignment()) }
	movl $2, %edx
	callq *__rustc::__rust_dealloc@GOTPCREL(%rip)
.LBB226_504:
	movq 40(%rsp), %rax
	movq %rax, 48(%rsp)
	movq 616(%rsp), %rsi
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hashbrown-0.16.1/src/raw/mod.rs:2312
		if !self.is_empty_singleton() {
	testq %rsi, %rsi
	je .LBB226_507
	jmp .LBB226_505
.LBB226_495:
	movq 48(%rsp), %rax
	movq %rax, 40(%rsp)
		// crates/impact_voxel/src/object/extraction.rs:1269
		}
	testb %bpl, %bpl
	jne .LBB226_499
.LBB226_494:
	movq 616(%rsp), %rsi
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hashbrown-0.16.1/src/raw/mod.rs:2312
		if !self.is_empty_singleton() {
	testq %rsi, %rsi
	je .LBB226_507
.LBB226_505:
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
	je .LBB226_507
	movq 776(%rsp), %rdi
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mut_ptr.rs:1072
		unsafe { intrinsics::offset(self, intrinsics::unchecked_sub(0, count as isize)) }
	subq %rax, %rdi
	.cfi_escape 0x2e, 0x00
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/alloc.rs:128
		unsafe { __rust_dealloc(ptr, layout.size(), layout.alignment()) }
	movl $16, %edx
	callq *__rustc::__rust_dealloc@GOTPCREL(%rip)
.LBB226_507:
		// crates/impact_voxel/src/object/extraction.rs:1269
		}
	movq 400(%rsp), %rax
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/raw_vec/mod.rs:634
		if elem_layout.size() == 0 || self.cap.as_inner() == 0 {
	testq %rax, %rax
	je .LBB226_509
		// crates/impact_voxel/src/object/extraction.rs:1269
		}
	movq 408(%rsp), %rdi
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/num/uint_macros.rs:1359
		intrinsics::unchecked_mul(self, rhs)
	leaq (%rax,%rax,2), %rsi
	.cfi_escape 0x2e, 0x00
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/alloc.rs:128
		unsafe { __rust_dealloc(ptr, layout.size(), layout.alignment()) }
	movl $1, %edx
	callq *__rustc::__rust_dealloc@GOTPCREL(%rip)
.LBB226_509:
		// crates/impact_voxel/src/object/extraction.rs:1269
		}
	movq 96(%rsp), %rsi
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/raw_vec/mod.rs:634
		if elem_layout.size() == 0 || self.cap.as_inner() == 0 {
	testq %rsi, %rsi
	je .LBB226_511
		// crates/impact_voxel/src/object/extraction.rs:1269
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
.LBB226_511:
	movq 48(%rsp), %rax
	xorl %ebp, %ebp
.LBB226_512:
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/allocator-api2-0.2.21/src/stable/raw_vec.rs:333
		if mem::size_of::<T>() == 0 || self.cap == 0 {
	testq %rax, %rax
	je .LBB226_515
		// crates/impact_alloc/src/arena.rs:169
		unsafe { &*self.arena_ptr }
	movq 72(%rsp), %rcx
	movq %rax, %rdx
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:2505
		Bump::<MIN_ALIGN>::dealloc(self, ptr, layout)
	movq 16(%rcx), %rax
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cell.rs:555
		unsafe { *self.value.get() }
	movq 32(%rax), %rcx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/non_null.rs:1714
		self.as_ptr() == other.as_ptr()
	cmpq 192(%rsp), %rcx
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:2231
		if self.is_last_allocation(ptr) {
	jne .LBB226_515
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/alloc/layout.rs:573
		if element_size != 0 && n > Layout::max_size_for_alignment(alignment) / element_size {
	shlq $4, %rdx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mut_ptr.rs:961
		unsafe { intrinsics::offset(self, count) }
	addq %rdx, %rcx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/mem/mod.rs:931
		crate::intrinsics::write_via_move(dest, src);
	movq %rcx, 32(%rax)
.LBB226_515:
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/allocator-api2-0.2.21/src/stable/raw_vec.rs:333
		if mem::size_of::<T>() == 0 || self.cap == 0 {
	cmpq $0, 136(%rsp)
	je .LBB226_518
.LBB226_516:
		// crates/impact_alloc/src/arena.rs:169
		unsafe { &*self.arena_ptr }
	movq 72(%rsp), %rax
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:2505
		Bump::<MIN_ALIGN>::dealloc(self, ptr, layout)
	movq 16(%rax), %rax
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cell.rs:555
		unsafe { *self.value.get() }
	movq 32(%rax), %rcx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/non_null.rs:1714
		self.as_ptr() == other.as_ptr()
	cmpq 184(%rsp), %rcx
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:2231
		if self.is_last_allocation(ptr) {
	jne .LBB226_518
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
.LBB226_518:
	movq 144(%rsp), %rax
	movq %rax, 40(%rsp)
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/allocator-api2-0.2.21/src/stable/raw_vec.rs:333
		if mem::size_of::<T>() == 0 || self.cap == 0 {
	testq %rax, %rax
	je .LBB226_521
.LBB226_519:
		// crates/impact_alloc/src/arena.rs:169
		unsafe { &*self.arena_ptr }
	movq 72(%rsp), %rax
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
	jne .LBB226_521
	movq 40(%rsp), %rdx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/alloc/layout.rs:573
		if element_size != 0 && n > Layout::max_size_for_alignment(alignment) / element_size {
	shlq $5, %rdx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mut_ptr.rs:961
		unsafe { intrinsics::offset(self, count) }
	addq %rdx, %rcx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/mem/mod.rs:931
		crate::intrinsics::write_via_move(dest, src);
	movq %rcx, 32(%rax)
.LBB226_521:
		// crates/impact_voxel/src/object/extraction.rs:1269
		}
	movq 80(%rsp), %rdi
	.cfi_escape 0x2e, 0x00
	callq core::ptr::drop_glue::<impact_alloc::arena::PoolArena>
.LBB226_522:
	testb %bpl, %bpl
	je .LBB226_538
	movq 424(%rsp), %rbx
	movq (%rbx), %rsi
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/raw_vec/mod.rs:634
		if elem_layout.size() == 0 || self.cap.as_inner() == 0 {
	testq %rsi, %rsi
	jne .LBB226_529
		// crates/impact_voxel/src/object/extraction.rs:1269
		}
	movq 24(%rbx), %rax
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/raw_vec/mod.rs:634
		if elem_layout.size() == 0 || self.cap.as_inner() == 0 {
	testq %rax, %rax
	jne .LBB226_530
.LBB226_525:
		// crates/impact_voxel/src/object/extraction.rs:1269
		}
	movq 152(%rbx), %rsi
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hashbrown-0.16.1/src/raw/mod.rs:2312
		if !self.is_empty_singleton() {
	testq %rsi, %rsi
	jne .LBB226_531
	jmp .LBB226_533
.LBB226_496:
		// crates/impact_alloc/src/arena.rs:169
		unsafe { &*self.arena_ptr }
	movq 72(%rsp), %rax
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:2505
		Bump::<MIN_ALIGN>::dealloc(self, ptr, layout)
	movq 16(%rax), %rax
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cell.rs:555
		unsafe { *self.value.get() }
	movq 32(%rax), %rcx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/non_null.rs:1714
		self.as_ptr() == other.as_ptr()
	cmpq 32(%rsp), %rcx
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bumpalo-3.20.2/src/lib.rs:2231
		if self.is_last_allocation(ptr) {
	jne .LBB226_498
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/alloc/layout.rs:573
		if element_size != 0 && n > Layout::max_size_for_alignment(alignment) / element_size {
	leaq (%r13,%r13,2), %rdx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mut_ptr.rs:961
		unsafe { intrinsics::offset(self, count) }
	leaq (%rcx,%rdx,8), %rcx
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/mem/mod.rs:931
		crate::intrinsics::write_via_move(dest, src);
	movq %rcx, 32(%rax)
.LBB226_498:
	movq 40(%rsp), %rax
	movq %rax, 48(%rsp)
		// crates/impact_voxel/src/object/extraction.rs:1269
		}
	testb %bpl, %bpl
	je .LBB226_494
	jmp .LBB226_499
.LBB226_526:
	.cfi_escape 0x2e, 0x00
	movq 784(%rsp), %rdi
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/alloc.rs:128
		unsafe { __rust_dealloc(ptr, layout.size(), layout.alignment()) }
	movl $1, %edx
	callq *__rustc::__rust_dealloc@GOTPCREL(%rip)
	movq 824(%rsp), %rsi
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/raw_vec/mod.rs:634
		if elem_layout.size() == 0 || self.cap.as_inner() == 0 {
	testq %rsi, %rsi
	je .LBB226_501
.LBB226_527:
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/num/uint_macros.rs:1359
		intrinsics::unchecked_mul(self, rhs)
	shlq $3, %rsi
	.cfi_escape 0x2e, 0x00
	movq 792(%rsp), %rdi
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/alloc.rs:128
		unsafe { __rust_dealloc(ptr, layout.size(), layout.alignment()) }
	movl $4, %edx
	callq *__rustc::__rust_dealloc@GOTPCREL(%rip)
	movq 832(%rsp), %rsi
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/raw_vec/mod.rs:634
		if elem_layout.size() == 0 || self.cap.as_inner() == 0 {
	testq %rsi, %rsi
	je .LBB226_502
.LBB226_528:
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/num/uint_macros.rs:1359
		intrinsics::unchecked_mul(self, rhs)
	addq %rsi, %rsi
	.cfi_escape 0x2e, 0x00
	movq 800(%rsp), %rdi
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/alloc.rs:128
		unsafe { __rust_dealloc(ptr, layout.size(), layout.alignment()) }
	movl $2, %edx
	callq *__rustc::__rust_dealloc@GOTPCREL(%rip)
	movq 840(%rsp), %rsi
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/raw_vec/mod.rs:634
		if elem_layout.size() == 0 || self.cap.as_inner() == 0 {
	testq %rsi, %rsi
	jne .LBB226_503
	jmp .LBB226_504
.LBB226_529:
		// crates/impact_voxel/src/object/extraction.rs:1269
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
		// crates/impact_voxel/src/object/extraction.rs:1269
		}
	movq 24(%rbx), %rax
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/raw_vec/mod.rs:634
		if elem_layout.size() == 0 || self.cap.as_inner() == 0 {
	testq %rax, %rax
	je .LBB226_525
.LBB226_530:
		// crates/impact_voxel/src/object/extraction.rs:1269
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
		// crates/impact_voxel/src/object/extraction.rs:1269
		}
	movq 152(%rbx), %rsi
		// ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hashbrown-0.16.1/src/raw/mod.rs:2312
		if !self.is_empty_singleton() {
	testq %rsi, %rsi
	je .LBB226_533
.LBB226_531:
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
	je .LBB226_533
		// crates/impact_voxel/src/object/extraction.rs:1269
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
.LBB226_533:
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mod.rs:825
		pub(crate) const unsafe fn drop_glue<T: PointeeSized>(_: &mut T)
	movq 48(%rbx), %rsi
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/raw_vec/mod.rs:634
		if elem_layout.size() == 0 || self.cap.as_inner() == 0 {
	testq %rsi, %rsi
	jne .LBB226_539
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mod.rs:825
		pub(crate) const unsafe fn drop_glue<T: PointeeSized>(_: &mut T)
	movq 72(%rbx), %rsi
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/raw_vec/mod.rs:634
		if elem_layout.size() == 0 || self.cap.as_inner() == 0 {
	testq %rsi, %rsi
	jne .LBB226_540
.LBB226_535:
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mod.rs:825
		pub(crate) const unsafe fn drop_glue<T: PointeeSized>(_: &mut T)
	movq 96(%rbx), %rsi
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/raw_vec/mod.rs:634
		if elem_layout.size() == 0 || self.cap.as_inner() == 0 {
	testq %rsi, %rsi
	jne .LBB226_541
.LBB226_536:
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mod.rs:825
		pub(crate) const unsafe fn drop_glue<T: PointeeSized>(_: &mut T)
	movq 120(%rbx), %rsi
		// ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/raw_vec/mod.rs:634
		if elem_layout.size() == 0 || self.cap.as_inner() == 0 {
	testq %rsi, %rsi
	je .LBB226_538
.LBB226_537:
	movq 424(%rsp), %rax
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
.LBB226_538:
	.cfi_escape 0x2e, 0x00
	movq %r15, %rdi
	callq _Unwind_Resume@PLT
.LBB226_539:
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
	je .LBB226_535
.LBB226_540:
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
	je .LBB226_536
.LBB226_541:
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
	jne .LBB226_537
	jmp .LBB226_538
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
	.asciz	",\000\000\000\000\000\000\000\273\004\000\000.\000\000"

.Lanon.5bbf1cbe9e9f837e6488bd4d315a6b9f.27:
	.quad	.Lanon.5bbf1cbe9e9f837e6488bd4d315a6b9f.14
	.asciz	",\000\000\000\000\000\000\000\257\004\000\000.\000\000"

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
