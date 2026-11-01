# 实验三扩展：Clock 页面置换

题目：https://github.com/rcore-os/rCore-Tutorial/blob/master/docs/lab-3/practice.md 。官方明确要求在 lab-3+ 基础上实现页面置换，本分支保留设备、SFS 和交换文件路径。

## 实现

SwapperImpl 改为 ClockSwapper，队首代表时钟指针。淘汰时检查实际页表项 ACCESSED：若为一则清零并给予第二次机会；若为零则移出。清 A 位后执行 sfence.vma，使硬件后续访问能再次更新 A 位。页表帧由 Mapping 持有且地址稳定，队列存放页表项地址，不拥有或释放它；unmap 通过 retain 删除引用。

每次 push/pop/retain 的正常复杂度是 O(1)，一次完整扫描 O(n)，最坏淘汰扫描两圈。相比 FIFO，Clock 可避免直接淘汰近期使用过的页，但在循环访问的工作集大于配额时仍会抖动，不能保证所有访问序列都优于 FIFO。

## 验证

clock_self_test 用真实 FrameTracker 和页表项验证跳过 A=1 页、清除 A 位、全 A=1 时的有界扫描、空队列及 retain。真实内核线程在仅 16 页物理配额下对 64 页栈数据进行三轮写入和逆序校验，统计实际缺页次数，要求超过 64 次，并输出 LAB3+ SUCCESS 后关机。

Actions 使用 tools/mkimage 建立真实 SFS 镜像及 16MiB SWAP_FILE，用 VirtIO 磁盘运行内核。QEMU 使用 legacy VirtIO MMIO 模式匹配同期驱动。QEMU 与 K210 专属模块通过已有 board feature 区分；当前验证目标是 QEMU，未声称 K210 硬件通过。本地不编译。

地址转换原理：启动页表保留恒等映射和高地址映射；页表帧与数据帧分别由 page_tables 与置换器管理。尚未映射到用户虚拟地址的物理页，仍可通过内核线性映射初始化。更完整的基础映射实验见 edidada/lab-3 的 REPORT.md。
