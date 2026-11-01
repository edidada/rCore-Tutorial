# 实验一：中断

题目来源：https://github.com/rcore-os/rCore-Tutorial/blob/master/docs/lab-1/practice.md 。本分支基于官方 lab-1 样例。

## 原理与分析

`ebreak` 进入中断后，汇编从 sp 减去 Context 大小，保存原寄存器和原 sp。处理函数的局部栈帧会继续改变 sp，但返回时平衡；恢复汇编最终从 Context 恢复原 sp，所以断点前后栈指针一致。

原样例若删除末尾 panic，rust_main 会返回到 entry.asm 中 jal 后面的地址；那里原本没有明确的后续指令，可能执行对齐填充或相邻函数，不能保证停止。本实现明确在返回位置执行 ld ra, 0(zero)，触发读取访问异常，避免将取指异常混为读取异常。

## 实现与验证

在异常分派中显式匹配 LoadFault，只有 stval 为零时打印 SUCCESS!，随后 panic 并通过 SBI shutdown。stval 是故障访问地址，sepc 是故障指令地址，二者不能混用。新增 Rust 实验代码没有 unsafe 块，原有 ebreak 和中断初始化 unsafe 块未修改。

Actions 使用原始 nightly-2020-06-27 构建 ELF 与 kernel.bin，并运行 scripts/test_lab1.py：必须观察到断点、SUCCESS!、LoadFault at 0x0 和正常关机。超时或其他未处理异常均失败。日志随构建产物上传。本地不编译、不运行 QEMU。
