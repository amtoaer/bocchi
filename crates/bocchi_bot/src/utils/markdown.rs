use anyhow::Result;

pub fn markdown_to_html(markdown: &str) -> String {
    // 可以使用 unwrap，因为 to_html_with_options 永远不会 Error
    markdown::to_html_with_options(markdown, &markdown::Options::gfm()).unwrap()
}

pub async fn html_to_image(html: &str) -> Result<Vec<u8>> {
    let c = fantoccini::ClientBuilder::native()
        .capabilities(serde_json::Map::from_iter(vec![(
            "moz:firefoxOptions".to_string(),
            serde_json::json!(
                {
                    "args": ["-headless", "--force-device-scale-factor=2.0"],
                    "prefs": {
                        "layout.css.devPixelsPerPx" : "2.5"
                    },
                }
            ),
        )]))
        .connect("http://localhost:4444")
        .await?;

    c.goto("about:blank").await?;
    c.execute(
        "document.body.innerHTML = arguments[0];",
        vec![serde_json::json!(html)],
    )
    .await?;
    let screenshot = c.screenshot().await?;
    Ok(screenshot)
}

#[cfg(test)]
mod tests {
    use std::{fs, path::Path};

    use super::*;

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
