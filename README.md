# SimpleFile

一个简单的rust std::fs::File实现

+ 以不同的模式打开文件
+ 读写操作
+ 文件资源管理，例如关闭
+ 基本的错误处理

实现一个类似于标准库提供的File等价的File无容置疑是复杂的，涉及到底层系统调用、跨平台兼容、错误处理等。本仓库将实现一个简化的版本.

> 假定目标是Unix-like OS，使用POSIX syscall. MacOS M1环境下开发测试


