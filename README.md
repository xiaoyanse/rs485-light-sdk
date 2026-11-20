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

## 使用边界

- `apply` 执行持续场景，业务程序负责何时切换和清除灯态。支持七色预设、闪烁、蜂鸣和全关，固件支持的组合以设备资料为准。
- `preview` 用于独占台架验装，最长保持五秒。即使场景写入结果不确定，也会尝试发全关。若无法确认全关，会明确返回 `reset_unconfirmed`。进程终止、断电或失去串口连接时，软件无法保证复位。
- 由单一进程持有串口。业务程序负责报警仲裁、工位绑定和试灯准入。首版覆盖串口直连，TCP 网关和 PWM 混色留待独立验证。

MIT 许可证。
