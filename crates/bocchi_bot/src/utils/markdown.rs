//! 目前的 html 渲染图片依赖 gecko 驱动与 firefox 浏览器，请确保安装后再使用

#![allow(unused)]

use std::io::{self, BufRead};
use std::process::{Child, Command};
use std::sync::{LazyLock, OnceLock};
use tokio::sync::OnceCell;

use anyhow::Result;

const PORT: &str = "4444";
const FIREFOX_BINARY: &str = "/usr/bin/firefox";
const GECKO_DRIVER_BINARY: &str = "/usr/bin/geckodriver";

static GECKO_DRIVER_COMMAND: LazyLock<Child> = LazyLock::new(|| {
    let mut gecko_driver_process = Command::new(GECKO_DRIVER_BINARY)
        .args(["--port", PORT, "--binary", FIREFOX_BINARY])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("启动 Gecko 驱动失败");
    match gecko_driver_process.stdout.take() {
        None => panic!("获取 Gecko 输出失败"),
        Some(stdout) => {
            let mut reader = io::BufReader::new(stdout);
            let mut buf = String::new();
            reader.read_line(&mut buf).expect("读取 Gecko 输出失败");
            // 目前很简单，当输出中包含监听地址时，认为启动成功
            if buf.contains(&format!("Listening on 127.0.0.1:{PORT}")) {
                // 重要：必须将 stdout 重新放回 gecko_driver_process，否则后续日志找不到 pipe 输出，process 会中断
                gecko_driver_process.stdout = Some(reader.into_inner());
                return gecko_driver_process;
            }
            panic!("启动 Gecko 驱动失败");
        }
    }
});

static FANTOCCINI_CLIENT: OnceCell<fantoccini::Client> = OnceCell::const_new();

pub fn markdown_to_html(markdown: &str) -> String {
    // 可以使用 unwrap，因为文档说 to_html_with_options 永远不会 Error
    markdown::to_html_with_options(markdown, &markdown::Options::gfm()).unwrap()
}

pub async fn html_to_image(html: &str) -> Result<Vec<u8>> {
    // client 构建必须要晚于 gecko 驱动的启动
    LazyLock::force(&GECKO_DRIVER_COMMAND);
    let browser = FANTOCCINI_CLIENT
        .get_or_init(|| async {
            fantoccini::ClientBuilder::native()
                .capabilities(serde_json::Map::from_iter(vec![(
                    "moz:firefoxOptions".to_string(),
                    serde_json::json!(
                        {
                            "args": ["-headless", "-width=1440", "-height=900"],
                            "prefs": {
                                "layout.css.devPixelsPerPx" : "2.5",
                            },
                        }
                    ),
                )]))
                .connect("http://localhost:4444")
                .await
                .expect("连接到 Gecko 驱动失败")
        })
        .await
        .clone();
    browser.goto("about:blank").await?;
    browser
        .execute(
            "document.body.innerHTML = arguments[0];",
            vec![serde_json::json!(html)],
        )
        .await?;
    let screenshot = browser.screenshot().await?;
    browser.close().await?;
    Ok(screenshot)
}

#[cfg(test)]
mod tests {
    use std::{fs, path::Path};

    use super::*;

    #[ignore = "only for debug"]
    #[tokio::test]
    async fn test_markdown_to_html() {
        let markdown = r#"
# 数学公式示例

这是一个包含数学公式的 Markdown 文件。

## 行内公式

这是一个行内公式示例：$E = mc^2$

## 块级公式

这是一个块级公式示例：

$$
\int_{a}^{b} f(x) \, dx = F(b) - F(a)
$$

## 其他公式

1. 勾股定理：$a^2 + b^2 = c^2$
2. 欧拉公式：$e^{i\pi} + 1 = 0$
3. 二次方程求根公式：$x = \frac{{-b \pm \sqrt{{b^2 - 4ac}}}}{2a}$

希望这些示例对你有帮助！
"#;
        let html = markdown_to_html(markdown);
        let image = html_to_image(html.as_str()).await.unwrap();
        fs::write(Path::new("./test.png"), image).unwrap();
    }
}
