use thiserror::Error;

pub type AppResult<T> = Result<T, AppError>;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("尚未设置项目根目录")]
    ProjectRootNotSet,

    #[error("项目根目录无效：缺少 {0}")]
    ProjectRootInvalid(String),

    #[error("读取文件失败：{path}：{detail}")]
    ReadFileFailed { path: String, detail: String },

    #[error("写入文件失败：{path}：{detail}")]
    WriteFileFailed { path: String, detail: String },

    #[error("解析 YAML 失败：{path}：{detail}")]
    ParseYamlFailed { path: String, detail: String },

    #[error("校验失败：{0}")]
    ValidationFailed(String),
}

impl AppError {
    pub fn read_file_failed(path: impl Into<String>, detail: impl ToString) -> Self {
        Self::ReadFileFailed {
            path: path.into(),
            detail: detail.to_string(),
        }
    }

    pub fn write_file_failed(path: impl Into<String>, detail: impl ToString) -> Self {
        Self::WriteFileFailed {
            path: path.into(),
            detail: detail.to_string(),
        }
    }

    pub fn parse_yaml_failed(path: impl Into<String>, detail: impl ToString) -> Self {
        Self::ParseYamlFailed {
            path: path.into(),
            detail: detail.to_string(),
        }
    }
}
