# RS485 Light SDK

通过 Modbus RTU 控制使用 `00C2` 预设寄存器的警示灯。Rust 原生库、Python 绑定和命令行验装共用同一套串口驱动。

## 验装工具

从仓库根目录执行。将示例中的 `COM3` 换成设备实际端口。

```powershell
cargo run -- ports
cargo run -- check COM3 FF 9600
cargo run -- preview COM3 red-steady
cargo run -- preview COM3 yellow-slow
cargo run -- preview COM3 green-steady
cargo run -- preview COM3 alarm-steady
cargo run -- off COM3
```

默认参数是 9600 baud、8N1、十六进制站号 `FF`。串口枚举列出系统端口，`check` 单独向指定地址读取 `00FF` 寄存器并验证回包，返回原始寄存器值。连接检查范围以支持该只读命令的固件为限。控制命令要求回显与请求完全一致。灯光和声音需要现场确认。

## Rust

```rust
use rs485_light_sdk::{Color, Light, Pattern, Scene, Station};
use std::time::Duration;

fn main() -> Result<(), rs485_light_sdk::Error> {
    let mut light = Light::open("COM3", 9600, Station::FACTORY, Duration::from_millis(750))?;
    let raw = light.verify_connection()?;
    println!("Register 00FF = {raw:04X}");
    light.preview(Scene::Light(Color::Green, Pattern::Steady), Duration::from_millis(750))?;
    Ok(())
}
```

依赖方可以使用本地 Cargo path 或 Git 固定提交。通过 `Error::kind()` 可分类处理串口打开失败、超时、无效回显和未确认的复位。

## Python 3.11+

Windows x64 可直接安装 [GitHub Release](https://github.com/xiaoyanse/rs485-light-sdk/releases) 中的 `cp311-abi3-win_amd64` wheel，无需编译。

从源码目录运行 `python -m pip install .` 生成并安装本机扩展，需要 Rust 工具链。

```python
from rs485_light_sdk import Light, list_ports

print(list_ports())
light = Light("COM3", station="FF", baud=9600)
try:
    print(f"{light.verify_connection():04X}")
    light.preview("alarm-steady", hold_ms=750)
finally:
    light.close()
```

## Windows x64 DLL

从 [GitHub Release](https://github.com/xiaoyanse/rs485-light-sdk/releases) 下载 DLL SDK 压缩包。内含 `rs485_light_sdk.dll`、MSVC 导入库和 C 头文件。C/C++ 链接导入库，把 DLL 放在应用程序旁。C# 或其他支持 C ABI 的语言可以按头文件声明直接加载 DLL。

解压后可用 MSVC x64 开发者终端编译示例：

```powershell
cl /nologo /W4 /I include examples\c\light_check.c /link /LIBPATH:. rs485_light_sdk.lib
.\light_check.exe COM3 FF
.\light_check.exe COM3 FF green-steady
```

调用方应核对 `rs485_light_abi_version()`，串行使用同一个设备句柄，并通过 `rs485_light_close` 释放端口。场景名称与 CLI 相同。函数返回固定状态码，`rs485_light_status_message` 提供说明。需要自己构建 DLL 时，使用 `cargo build --release --locked --lib --features c-api`。

## 使用边界

- `apply` 执行持续场景，业务程序负责何时切换和清除灯态。支持七色预设、闪烁、蜂鸣和全关，固件支持的组合以设备资料为准。
- `preview` 用于独占台架验装，最长保持五秒。即使场景写入结果不确定，也会尝试发全关。若无法确认全关，会明确返回 `reset_unconfirmed`。进程终止、断电或失去串口连接时，软件无法保证复位。
- 由单一进程持有串口。业务程序负责报警仲裁、工位绑定和试灯准入。首版覆盖串口直连，TCP 网关和 PWM 混色留待独立验证。

MIT 许可证。
