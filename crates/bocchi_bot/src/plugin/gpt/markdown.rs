//! 目前的 html 渲染图片依赖运行在本机的 gecko 驱动与 firefox 浏览器，请确保安装后再使用
//! gecko 驱动与浏览器会由程序自动启动，只需要提前安装好并指定路径即可
use std::sync::LazyLock;

use aho_corasick::AhoCorasick;
use anyhow::Result;
use async_tempfile::TempFile;
use fantoccini::Locator;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::Command,
    sync::OnceCell,
};

const PORT: &str = "4444";
const FIREFOX_BINARY: &str = "/usr/bin/firefox";
const GECKO_DRIVER_BINARY: &str = "/usr/bin/geckodriver";

static GECKO_DRIVER_INITED: OnceCell<()> = OnceCell::const_new();
static FANTOCCINI_CLIENT: OnceCell<fantoccini::Client> = OnceCell::const_new();
static AHO_CORASICK: LazyLock<AhoCorasick> = LazyLock::new(|| AhoCorasick::new([r"\[", r"\]", r"\(", r"\)"]).unwrap());

async fn run_gecko_driver() {
    let mut gecko_driver_process = Command::new(GECKO_DRIVER_BINARY)
        .args(["--port", PORT, "--binary", FIREFOX_BINARY])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .expect("启动 Gecko 驱动失败");
    match gecko_driver_process.stdout.take() {
        None => panic!("获取 Gecko 输出失败"),
        Some(stdout) => {
            let mut reader = BufReader::new(stdout);
            let mut buf = String::new();
            reader.read_line(&mut buf).await.expect("读取 Gecko 输出失败");
            // 目前很简单，当输出中包含监听地址时，认为启动成功
            if !buf.contains(&format!("Listening on 127.0.0.1:{PORT}")) {
                panic!("启动 Gecko 驱动失败");
            }
            // 重要：必须将 stdout 重新放回 gecko_driver_process，否则后续日志找不到 pipe 输出，process 会中断
            gecko_driver_process.stdout = Some(reader.into_inner());
            // 阻塞等待 gecko_driver 退出
            tokio::spawn(async move {
                gecko_driver_process.wait().await.expect("Gecko 驱动异常退出");
            });
        }
    }
}

pub async fn markdown_to_image(markdown: String) -> Result<Vec<u8>> {
    let html = markdown_to_html(markdown).await?;
    html_to_image(&html).await
}

async fn markdown_to_html(markdown: String) -> Result<String> {
    Ok(tokio::task::spawn_blocking(move || {
        // 将 \[、\] 替换为 $$，\(、\) 替换为 $，这是 Katex 的默认兼容写法，与 pulldown_cmark 也不会有冲突。
        let markdown = AHO_CORASICK.replace_all(&markdown, &["$$", "$$", "$", "$"]);
        let parser = pulldown_cmark::Parser::new_ext(&markdown, pulldown_cmark::Options::ENABLE_TABLES);
        let mut html = String::new();
        pulldown_cmark::html::push_html(&mut html, parser);
        html
    })
    .await?)
}

async fn html_to_image(html: &str) -> Result<Vec<u8>> {
    GECKO_DRIVER_INITED.get_or_init(run_gecko_driver).await;
    // client 构建必须要晚于 gecko 驱动的启动
    let browser = FANTOCCINI_CLIENT
        .get_or_init(|| async {
            fantoccini::ClientBuilder::rustls()
                .unwrap()
                .capabilities(serde_json::Map::from_iter(vec![(
                    "moz:firefoxOptions".to_string(),
                    serde_json::json!(
                        {
                            "args": ["-headless", "-width=1440", "-height=900"],
                            "prefs": {
                                "layout.css.devPixelsPerPx" : "2.0",
                            },
                        }
                    ),
                )]))
                .connect(&format!("http://localhost:{PORT}"))
                .await
                .expect("连接到 Gecko 驱动失败")
        })
        .await
        .clone();
    let mut tempfile = TempFile::new().await?;
    tempfile.write_all(render(html).as_bytes()).await?;
    tempfile.flush().await?;
    browser
        .goto(&format!("file://{}", tempfile.file_path().to_string_lossy()))
        .await?;
    let article = browser.wait().for_element(Locator::Css(".markdown-body")).await?;
    let screenshot = article.screenshot().await?;
    Ok(screenshot)
}

fn render(html: &str) -> String {
    format!(
        r#"
<!DOCTYPE html>
<html>

<head>
    <link rel="stylesheet" href="https://cdnjs.cloudflare.com/ajax/libs/highlight.js/11.9.0/styles/github.min.css">
    <script src="https://cdnjs.cloudflare.com/ajax/libs/highlight.js/11.9.0/highlight.min.js"></script>
    <link rel="stylesheet"
        href="https://cdnjs.cloudflare.com/ajax/libs/github-markdown-css/5.6.1/github-markdown.min.css">
    <style>
        .markdown-body {{
            --fontStack-monospace: JetBrains Mono, ui-monospace,
                SFMono-Regular,
                SF Mono,
                Menlo,
                Consolas,
                Liberation Mono,
                monospace !important;
    }}

        .markdown-body {{
            box-sizing: border-box;
            min-width: 200px;
            max-width: 980px;
            margin: 0 auto;
            padding: 45px;
    }}

        @media (max-width: 767px) {{
            .markdown-body {{
                padding: 15px;
            }}
        }}
    </style>
    <script>
        hljs.highlightAll();
    </script>
    <link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/katex@0.16.11/dist/katex.min.css"
        integrity="sha384-nB0miv6/jRmo5UMMR1wu3Gz6NLsoTkbqJghGIsx//Rlm+ZU03BU6SQNC66uf4l5+" crossorigin="anonymous">

    <!-- The loading of KaTeX is deferred to speed up page rendering -->
    <script defer src="https://cdn.jsdelivr.net/npm/katex@0.16.11/dist/katex.min.js"
        integrity="sha384-7zkQWkzuo3B5mTepMUcHkMB5jZaolc2xDwL6VFqjFALcbeS9Ggm/Yr2r3Dy4lfFg"
        crossorigin="anonymous"></script>

    <!-- To automatically render math in text elements, include the auto-render extension: -->
    <script defer src="https://cdn.jsdelivr.net/npm/katex@0.16.11/dist/contrib/auto-render.min.js"
        integrity="sha384-43gviWU0YVjaDtb/GhzOouOXtZMP/7XUzwPTstBeZFe/+rCMvRwr4yROQP43s0Xk" crossorigin="anonymous"
        onload="renderMathInElement(document.body, {{
            delimiters: [
                {{ left: '$$', right: '$$', display: true }},
                {{ left: '$', right: '$', display: false }},
            ]
        }});"></script>
</head>

<body class="markdown-body">
{}
</body>

</html>
"#,
        html
    )
}
