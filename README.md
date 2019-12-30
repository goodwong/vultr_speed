
一个vultr各机房测速工具
====================

> 学习rust过程的产物，用到了`tokio`/`reqwest`

背景
----
想购买一个vultr主机用于自己上网的需求，但是不知道选哪个机房好，于是首先想到用`ping`和`curl`两个工具测试丢包率、RTT、和下载速度；

然而测试几次发现，跨洋网络不稳定，可能上午发现纽约机房速度最佳，下午再测就变渣了 -_!

于是寻找连续测速的工具，并最终演变为自己开发。



原理
----
首先想到的是16个机房同时发起下载请求，写出来后发现，可能由于本地带宽不足，各个机房之间的速度会相互影响。

所以改为现在的方式：
1. 轮流每个机房发起一个下载请求；
2. 除去最开始的10s数据（这部分数据波动大）
3. 截取10s下载数据，计算平均值
4. 断开连接
5. 下一个机房，重复步骤1~5

如此长期运行，可以获得1天内的网络速率波动图（TUI界面）
![输出结果实例](https://github.com/goodwong/vultr_speed/raw/master/docs/vultr_output.png)


使用
---
1. Linux(Debian)系统下
2. 打开Terminal窗口，并将窗口宽度设置为不小于210列（否则会显示错乱）
3. 运行 vultr_speed，建议持续1天，以便观察各时间段网络速率变化情况



todo：（欢迎参与）
---------------
1. 输出速率曲线到图片格式（TUI变现力不足）
2. 可以自定义测试节点（通过读取nodes.txt）
3. 加入丢包率的连续测试（ICMP协议）



交叉编译windows目标（Debian）
-----------------
1. 首先，修改termion为支持windows版本的
    > 参考：https://gitlab.redox-os.org/redox-os/termion/issues/103#note_6722
    ```toml
    [dependencies]
    ....
    termion = { git = "https://github.com/mcgoo/termion", branch = "windows" }
    ```
2. 修改 src/main.rs：
    ```rust
    # 在 main() 里第一行加入：
    let _guard = termion::init();
    ```


3. 安装配置
    ```sh
    # 安装编译工具
    apt-get update
    apt-get install gcc-mingw-w64-x86-64 -y

    # 配置rustup
    mkdir ~/.cargo/
    tee -a ~/.cargo/config << END
    [target.x86_64-pc-windows-gnu]
    linker = "x86_64-w64-mingw32-gcc"
    ar = "x86_64-w64-mingw32-gcc-ar"
    END
    # 或者在当期项目根目录下 添加 .cargo/config文件，输入以上内容也可以

    rustup target add x86_64-pc-windows-gnu

    # 还需要修复一个bug，很多人会被这个坑了
    # 参考：https://blog.nanpuyue.com/2019/052.html
    # 简单的说就是 Rust 工具链中自带的 crt2.o 太老了，我们替换一下：
    cp -vb /usr/x86_64-w64-mingw32/lib/crt2.o "$(rustc --print sysroot)"/lib/rustlib/x86_64-pc-windows-gnu/lib/

    # 愉快的编译了
    # 产出文件：target/x86_64-pc-windows-gnu/release/vultr_speed.exe 拷贝到windows下，cmd运行
    cargo build --target=x86_64-pc-windows-gnu --release
    ```
4. 注意事项：
    1. windows只有cmd可以输出颜色，PowerShell没有颜色显示
    2. 如果cmd窗口启用了快速编辑模式，当鼠标选中文本时，输出会被中断（猜测抢不到读写锁）