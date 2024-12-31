use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "snake_case", tag = "type", content = "data")]
pub enum MessageSegment {
    /// 纯文本内容
    Text {
        /// 纯文本内容
        text: String,
    },
    /// QQ 表情
    Face {
        /// QQ 表情 ID
        id: String,
    },
    /// 图片
    Image {
        /// 图片文件名
        file: String,
        /// 图片类型，`flash` 表示闪照，无此参数表示普通图片
        #[serde(skip_serializing_if = "Option::is_none")]
        #[serde(default)]
        r#type: Option<String>,
        /// 图片 URL，仅接收
        #[serde(skip_serializing_if = "Option::is_none")]
        #[serde(default)]
        url: Option<String>,
        /// 只在通过网络 URL 发送时有效，表示是否使用已缓存的文件，默认 `true`，仅发送
        #[serde(skip_serializing_if = "Option::is_none")]
        #[serde(default)]
        cache: Option<bool>,
        /// 只在通过网络 URL 发送时有效，表示是否通过代理下载文件（需通过环境变量或配置文件配置代理），默认 `true`，仅发送
        #[serde(skip_serializing_if = "Option::is_none")]
        #[serde(default)]
        proxy: Option<bool>,
        /// 只在通过网络 URL 发送时有效，单位秒，表示下载网络文件的超时时间，默认不超时，仅发送
        #[serde(skip_serializing_if = "Option::is_none")]
        #[serde(default)]
        timeout: Option<u64>,
    },
    /// 语音
    Record {
        /// 语音文件名
        file: String,
        /// 发送时可选，默认 `false`，设置为 `true` 表示变声
        #[serde(skip_serializing_if = "Option::is_none")]
        #[serde(default)]
        magic: Option<bool>,
        /// 语音 URL，仅接收
        #[serde(skip_serializing_if = "Option::is_none")]
        #[serde(default)]
        url: Option<String>,
        /// 只在通过网络 URL 发送时有效，表示是否使用已缓存的文件，默认 `true`，仅发送
        #[serde(skip_serializing_if = "Option::is_none")]
        #[serde(default)]
        cache: Option<bool>,
        /// 只在通过网络 URL 发送时有效，表示是否通过代理下载文件（需通过环境变量或配置文件配置代理），默认 `true`，仅发送
        #[serde(skip_serializing_if = "Option::is_none")]
        #[serde(default)]
        proxy: Option<bool>,
        /// 只在通过网络 URL 发送时有效，单位秒，表示下载网络文件的超时时间，默认不超时，仅发送
        #[serde(skip_serializing_if = "Option::is_none")]
        #[serde(default)]
        timeout: Option<u64>,
    },
    /// 短视频
    Video {
        /// 视频文件名
        file: String,
        /// 视频 URL，仅接收
        #[serde(skip_serializing_if = "Option::is_none")]
        #[serde(default)]
        url: Option<String>,
        /// 只在通过网络 URL 发送时有效，表示是否使用已缓存的文件，默认 `true`，仅发送
        #[serde(skip_serializing_if = "Option::is_none")]
        #[serde(default)]
        cache: Option<bool>,
        /// 只在通过网络 URL 发送时有效，表示是否通过代理下载文件（需通过环境变量或配置文件配置代理），默认 `true`，仅发送
        #[serde(skip_serializing_if = "Option::is_none")]
        #[serde(default)]
        proxy: Option<bool>,
        /// 只在通过网络 URL 发送时有效，单位秒，表示下载网络文件的超时时间，默认不超时，仅发送
        #[serde(skip_serializing_if = "Option::is_none")]
        #[serde(default)]
        timeout: Option<u64>,
    },
    /// @某人
    At {
        /// @的 QQ 号，`all` 表示全体成员
        qq: String,
    },
    /// 猜拳魔法表情
    Rps,
    /// 掷骰子魔法表情
    Dice,
    /// 窗口抖动（戳一戳）
    Shake,
    /// 戳一戳
    Poke {
        /// 类型
        r#type: String,
        /// ID
        id: String,
        /// 表情名，仅接收
        #[serde(skip_serializing_if = "Option::is_none")]
        #[serde(default)]
        name: Option<String>,
    },
    /// 匿名发消息
    Anonymous {
        /// 可选，表示无法匿名时是否继续发送，仅发送
        #[serde(skip_serializing_if = "Option::is_none")]
        #[serde(default)]
        ignore: Option<bool>,
    },
    /// 链接分享
    Share {
        /// URL
        url: String,
        /// 标题
        title: String,
        /// 发送时可选，内容描述
        #[serde(skip_serializing_if = "Option::is_none")]
        #[serde(default)]
        content: Option<String>,
        /// 发送时可选，图片 URL
        #[serde(skip_serializing_if = "Option::is_none")]
        #[serde(default)]
        image: Option<String>,
    },
    /// 推荐好友
    Contact {
        /// qq/group
        r#type: String,
        /// 被推荐对象的 QQ 号或群号，根据 type 而定
        id: String,
    },
    /// 位置
    Location {
        /// 纬度
        lat: String,
        /// 经度
        lon: String,
        /// 发送时可选，标题
        #[serde(skip_serializing_if = "Option::is_none")]
        #[serde(default)]
        title: Option<String>,
        /// 发送时可选，内容描述
        #[serde(skip_serializing_if = "Option::is_none")]
        #[serde(default)]
        content: Option<String>,
    },
    /// 音乐分享
    Music {
        /// qq、163、xm，分别表示使用 QQ 音乐、网易云音乐、虾米音乐
        r#type: String,
        /// 歌曲 ID
        id: String,
    },
    /// 音乐自定义分享
    #[serde(rename = "music")]
    CustomMusic {
        /// 表示音乐自定义分享
        r#type: String,
        /// 点击后跳转目标 URL
        url: String,
        /// 音乐 URL
        audio: String,
        /// 标题
        title: String,
        /// 发送时可选，内容描述
        #[serde(skip_serializing_if = "Option::is_none")]
        #[serde(default)]
        content: Option<String>,
        /// 发送时可选，图片 URL
        #[serde(skip_serializing_if = "Option::is_none")]
        #[serde(default)]
        image: Option<String>,
    },
    /// 回复
    Reply {
        /// 回复时引用的消息 ID
        id: String,
    },
    /// 合并转发
    Forward {
        /// 合并转发 ID，需通过 `get_forward_msg` API 获取具体内容，仅接收
        id: String,
    },
    /// 合并转发节点
    Node {
        /// 转发的消息 ID
        #[serde(skip_serializing_if = "Option::is_none")]
        #[serde(default)]
        id: Option<String>,
        /// 发送者 QQ 号
        #[serde(skip_serializing_if = "Option::is_none")]
        #[serde(default)]
        user_id: Option<String>,
        /// 发送者昵称
        #[serde(skip_serializing_if = "Option::is_none")]
        #[serde(default)]
        nickname: Option<String>,
        /// 消息内容
        #[serde(skip_serializing_if = "Option::is_none")]
        #[serde(default)]
        content: Option<MessageContent>,
    },
    /// XML 消息
    Xml {
        /// XML 内容
        data: String,
    },
    /// JSON 消息
    Json {
        /// JSON 内容
        data: String,
    },
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
pub enum MessageContent {
    Text(String),
    Segment(Vec<MessageSegment>),
}
