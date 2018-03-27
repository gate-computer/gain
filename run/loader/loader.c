// Copyright (c) 2016 Timo Savola. All rights reserved.
// Use of this source code is governed by a BSD-style
// license that can be found in the LICENSE file.

#include <signal.h>
#include <stddef.h>
#include <stdint.h>

#include <sys/mman.h>
#include <sys/prctl.h>
#include <sys/syscall.h>
#include <sys/types.h>

#include "../defs.h"

#define xstr(s) str(s)
#define str(s) #s

#define NORETURN __attribute__((noreturn))
#define PACKED __attribute__((packed))

#define SYS_SA_RESTORER 0x04000000
#define SIGACTION_FLAGS (SA_RESTART | SYS_SA_RESTORER)

// avoiding function prototypes avoids GOT section
extern int runtime_exit;
extern int runtime_start_with_syscall;
extern int signal_handler;
extern int signal_restorer;
extern int trap_handler;

static int sys_personality(unsigned long persona)
{
	int retval;

	asm volatile(
		"syscall"
		: "=a"(retval)
		: "a"(SYS_personality), "D"(persona)
		: "cc", "rcx", "r11", "memory");

	return retval;
}

static int sys_prctl(int option, unsigned long arg2)
{
	int retval;

	asm volatile(
		"syscall"
		: "=a"(retval)
		: "a"(SYS_prctl), "D"(option), "S"(arg2)
		: "cc", "rcx", "r11", "memory");

	return retval;
}

static ssize_t sys_read(int fd, void *buf, size_t count)
{
	ssize_t retval;

	asm volatile(
		"syscall"
		: "=a"(retval)
		: "a"(SYS_read), "D"(fd), "S"(buf), "d"(count)
		: "cc", "rcx", "r11", "memory");

	return retval;
}

static void *sys_mmap(
	void *addr, size_t length, int prot, int flags, int fd, off_t offset)
{
	void *retval;

	register void *rdi asm("rdi") = addr;
	register size_t rsi asm("rsi") = length;
	register int rdx asm("rdx") = prot;
	register int r10 asm("r10") = flags;
	register int r8 asm("r8") = fd;
	register off_t r9 asm("r9") = offset;

	asm volatile(
		"syscall"
		: "=a"(retval)
		: "a"(SYS_mmap), "r"(rdi), "r"(rsi), "r"(rdx), "r"(r10), "r"(r8), "r"(r9)
		: "cc", "rcx", "r11", "memory");

	return retval;
}

static int sys_close(int fd)
{
	int retval;

	asm volatile(
		"syscall"
		: "=a"(retval)
		: "a"(SYS_close), "D"(fd)
		: "cc", "rcx", "r11", "memory");

	return retval;
}

static int read_full(void *buf, size_t size)
{
	for (size_t pos = 0; pos < size;) {
		ssize_t len = sys_read(0, buf + pos, size - pos);
		if (len < 0)
			return -1;
		pos += len;
	}

	return 0;
}

NORETURN
static void enter(
	uint64_t page,
	void *text_ptr,
	void *memory_ptr,
	void *init_memory_limit,
	void *grow_memory_limit,
	void *stack_ptr,
	void *stack_limit,
	int32_t arg)
{
	register void *rax asm("rax") = stack_ptr;
	register void *rdx asm("rdx") = &trap_handler;
	register void *rcx asm("rcx") = grow_memory_limit;
	register uint64_t rsi asm("rsi") = (GATE_LOADER_STACK_SIZE + page - 1) & ~(page - 1);
	register int64_t rdi asm("rdi") = arg;
	register void *r9 asm("r9") = &signal_handler;
	register void *r10 asm("r10") = &signal_restorer;
	register uint64_t r11 asm("r11") = page;
	register void *r12 asm("r12") = text_ptr;
	register void *r13 asm("r13") = stack_limit;
	register void *r14 asm("r14") = memory_ptr;
	register void *r15 asm("r15") = init_memory_limit;

	asm volatile(
		// clang-format off
		// runtime MMX registers
		"        movq    %%rdx, %%mm0                            \n" // trap handler
		"        movq    %%rcx, %%mm1                            \n" // grow memory limit
		"        movq    %%rdi, %%mm6                            \n" // arg
		// replace stack
		"        mov     %%rax, %%rsp                            \n"
		// load the stack ptr saved in _start, and unmap old stack (ASLR breaks this)
		"        movq    %%mm7, %%rdi                            \n" // ptr = stack top
		"        dec     %%r11                                   \n" // page-1
		"        add     %%r11, %%rdi                            \n" // ptr += page-1
		"        not     %%r11                                   \n" // ~(page-1)
		"        and     %%r11, %%rdi                            \n" // ptr &= ~(page-1)
		"        sub     %%rsi, %%rdi                            \n" // ptr -= stack size
		"        mov     $"xstr(SYS_munmap)", %%eax              \n"
		"        syscall                                         \n"
		"        mov     $58, %%edi                              \n"
		"        test    %%rax, %%rax                            \n"
		"        jne     runtime_exit                            \n"
		// register suspend signal handler (using 32 bytes of stack red zone)
		"        mov     $"xstr(SIGUSR1)", %%edi                 \n" // signum
		"        xor     %%edx, %%edx                            \n" // oldact
		"        lea     -32(%%rsp), %%rsi                       \n" // act
		"        mov     %%r9, (%%rsi)                           \n" //   handler
		"        movq    $"xstr(SIGACTION_FLAGS)", 8(%%rsi)      \n" //   flags
		"        mov     %%r10, 16(%%rsi)                        \n" //   restorer
		"        mov     %%rdx, 24(%%rsi)                        \n" //   mask (0)
		"        mov     $8, %%r10                               \n" // mask size
		"        xor     %%r9d, %%r9d                            \n" // clear suspend flag
		"        mov     $"xstr(SYS_rt_sigaction)", %%eax        \n"
		"        syscall                                         \n"
		"        mov     $59, %%edi                              \n"
		"        test    %%rax, %%rax                            \n"
		"        jne     runtime_exit                            \n"
		// execute runtime, which immediately makes syscall with these parameters
		"        mov     $"xstr(GATE_LOADER_TEXT_ADDR)", %%rdi   \n"
		"        mov     $"xstr(GATE_LOADER_TEXT_SIZE)", %%esi   \n"
		"        mov     $"xstr(SYS_munmap)", %%eax              \n"
		"        jmp     runtime_start_with_syscall              \n"
		// clang-format on
		:
		: "r"(rax), "r"(rdx), "r"(rcx), "r"(rsi), "r"(rdi), "r"(r9), "r"(r10), "r"(r11), "r"(r12), "r"(r13), "r"(r14), "r"(r15));

	__builtin_unreachable();
}

int main(int argc, char **argv, char **envp)
{
	if (sys_prctl(PR_SET_DUMPABLE, 0) != 0)
		return 48;

	// undo the personality change by executor.c
	if (sys_personality(0) < 0)
		return 49;

	// this is like imageInfo in run.go
	struct PACKED {
		uint64_t text_addr;
		uint64_t heap_addr;
		uint64_t stack_addr;
		uint32_t page_size;
		uint32_t rodata_size;
		uint32_t text_size;
		uint32_t globals_size;
		uint32_t init_memory_size;
		uint32_t grow_memory_size;
		uint32_t stack_size;
		uint32_t magic_number;
		int32_t arg;
	} info;

	if (read_full(&info, sizeof info) != 0)
		return 50;

	if (info.magic_number != GATE_MAGIC_NUMBER)
		return 51;

	if (info.rodata_size > 0) {
		void *ptr = sys_mmap((void *) GATE_RODATA_ADDR, info.rodata_size, PROT_READ, MAP_PRIVATE | MAP_FIXED | MAP_NORESERVE, GATE_MAPS_FD, 0);
		if (ptr != (void *) GATE_RODATA_ADDR)
			return 52;
	}

	void *text_ptr = sys_mmap((void *) info.text_addr, info.text_size, PROT_EXEC, MAP_PRIVATE | MAP_NORESERVE | MAP_FIXED, GATE_MAPS_FD, info.rodata_size);
	if (text_ptr != (void *) info.text_addr)
		return 53;

	size_t globals_memory_offset = (size_t) info.rodata_size + (size_t) info.text_size;
	size_t globals_memory_size = info.globals_size + info.grow_memory_size;

	void *memory_ptr = NULL;

	if (globals_memory_size > 0) {
		void *ptr = sys_mmap((void *) info.heap_addr, globals_memory_size, PROT_READ | PROT_WRITE, MAP_SHARED | MAP_FIXED | MAP_NORESERVE, GATE_MAPS_FD, globals_memory_offset);
		if (ptr != (void *) info.heap_addr)
			return 54;

		memory_ptr = ptr + info.globals_size;
	}

	void *init_memory_limit = memory_ptr + info.init_memory_size;
	void *grow_memory_limit = memory_ptr + info.grow_memory_size;

	size_t stack_offset = globals_memory_offset + globals_memory_size;

	void *stack_buf = sys_mmap((void *) info.stack_addr, info.stack_size, PROT_READ | PROT_WRITE, MAP_SHARED | MAP_FIXED | MAP_NORESERVE, GATE_MAPS_FD, stack_offset);
	if (stack_buf != (void *) info.stack_addr)
		return 55;

	void *stack_limit = stack_buf + GATE_SIGNAL_STACK_RESERVE;
	void *stack_ptr = stack_buf + info.stack_size;

	if (sys_close(GATE_MAPS_FD) != 0)
		return 56;

	enter(info.page_size, text_ptr, memory_ptr, init_memory_limit, grow_memory_limit, stack_ptr, stack_limit, info.arg);
}
