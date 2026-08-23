//! 知识库远程命令。

use std::io::{Read, Write};

use anyhow::{Context, Result};
use clap::{Subcommand, ValueEnum};
use serde_json::json;
use vw_gateway_client::vw_api_types::knowledge::{
    KnowledgeDatasetCreateRequest, KnowledgeDocumentCreateRequest, KnowledgeDocumentImportRequest,
    KnowledgeDocumentListQuery, KnowledgeDocumentPatchRequest, KnowledgeDocumentReindexRequest,
    KnowledgeDocumentStatus, KnowledgeDocumentsBatchActionRequest, KnowledgeEmbeddingTestRequest,
    KnowledgeRetrieveRequest,
};

use super::context::ApiContext;
use super::input::read_json_input;
use super::output;

#[derive(Subcommand, Debug)]
pub(super) enum KnowledgeCommands {
    /// 查询知识库运行状态。
    Status,
    /// 查询或测试嵌入配置。
    Embedding {
        #[command(subcommand)]
        command: EmbeddingCommands,
    },
    /// 管理知识数据集。
    Dataset {
        #[command(subcommand)]
        command: DatasetCommands,
    },
    /// 管理知识文档。
    Document {
        #[command(subcommand)]
        command: DocumentCommands,
    },
    /// 查询异步索引任务。
    Job {
        #[arg(value_parser = required)]
        job_id: String,
    },
    /// 执行知识检索。
    Retrieve {
        #[arg(long, value_parser = required)]
        input: String,
    },
    /// 执行有成本的检索测试。
    RetrievalTest {
        #[arg(long, value_parser = required)]
        input: String,
    },
}

#[derive(Subcommand, Debug)]
pub(super) enum EmbeddingCommands {
    Status,
    Presets,
    Test {
        #[arg(long, value_parser = required)]
        input: String,
    },
}

#[derive(Subcommand, Debug)]
pub(super) enum DatasetCommands {
    List,
    Create {
        #[arg(long, value_parser = required)]
        input: String,
    },
    Detail {
        #[arg(value_parser = required)]
        dataset_id: String,
    },
    Delete {
        #[arg(value_parser = required)]
        dataset_id: String,
    },
}

#[derive(Subcommand, Debug)]
pub(super) enum DocumentCommands {
    List {
        #[arg(value_parser = required)]
        dataset_id: String,
        #[arg(long, value_enum)]
        status: Option<DocumentStatusArg>,
    },
    Create {
        #[arg(value_parser = required)]
        dataset_id: String,
        #[arg(long, value_parser = required)]
        input: String,
    },
    Import {
        #[arg(value_parser = required)]
        dataset_id: String,
        #[arg(long, value_parser = required)]
        input: String,
    },
    ImportAsync {
        #[arg(value_parser = required)]
        dataset_id: String,
        #[arg(long, value_parser = required)]
        input: String,
    },
    Batch {
        #[arg(long, value_parser = required)]
        input: String,
    },
    Update {
        #[arg(value_parser = required)]
        document_id: String,
        #[arg(long, value_parser = required)]
        input: String,
    },
    Delete {
        #[arg(value_parser = required)]
        document_id: String,
    },
    Chunks {
        #[arg(value_parser = required)]
        document_id: String,
    },
    Reindex {
        #[arg(value_parser = required)]
        document_id: String,
        #[arg(long, value_parser = required)]
        input: String,
    },
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub(super) enum DocumentStatusArg {
    Pending,
    Processing,
    Completed,
    Failed,
    Disabled,
    Archived,
}

impl From<DocumentStatusArg> for KnowledgeDocumentStatus {
    fn from(status: DocumentStatusArg) -> Self {
        match status {
            DocumentStatusArg::Pending => Self::Pending,
            DocumentStatusArg::Processing => Self::Processing,
            DocumentStatusArg::Completed => Self::Completed,
            DocumentStatusArg::Failed => Self::Failed,
            DocumentStatusArg::Disabled => Self::Disabled,
            DocumentStatusArg::Archived => Self::Archived,
        }
    }
}

impl KnowledgeCommands {
    pub(super) fn dangerous_operation(&self) -> Option<String> {
        match self {
            Self::Embedding { command: EmbeddingCommands::Test { .. } } => {
                Some("test knowledge embedding configuration".to_string())
            }
            Self::Dataset { command: DatasetCommands::Delete { dataset_id } } => {
                Some(format!("delete knowledge dataset '{dataset_id}'"))
            }
            Self::Document {
                command:
                    DocumentCommands::Import { dataset_id, .. }
                    | DocumentCommands::ImportAsync { dataset_id, .. },
            } => Some(format!("import knowledge document into dataset '{dataset_id}'")),
            Self::Document { command: DocumentCommands::Batch { .. } } => {
                Some("batch update knowledge documents".to_string())
            }
            Self::Document { command: DocumentCommands::Delete { document_id } } => {
                Some(format!("delete knowledge document '{document_id}'"))
            }
            Self::Document { command: DocumentCommands::Reindex { document_id, .. } } => {
                Some(format!("reindex knowledge document '{document_id}'"))
            }
            Self::RetrievalTest { .. } => Some("test knowledge retrieval".to_string()),
            _ => None,
        }
    }
}

pub(super) async fn run(
    command: KnowledgeCommands,
    context: &ApiContext,
    stdin: &mut impl Read,
    writer: &mut impl Write,
) -> Result<()> {
    let client = context.client();
    let response = match command {
        KnowledgeCommands::Status => json!(client.knowledge_status().await.map_err(anyhow::Error::msg)?),
        KnowledgeCommands::Embedding { command } => match command {
            EmbeddingCommands::Status => json!(client.knowledge_embedding_status().await.map_err(anyhow::Error::msg)?),
            EmbeddingCommands::Presets => json!(client.knowledge_embedding_presets().await.map_err(anyhow::Error::msg)?),
            EmbeddingCommands::Test { input } => {
                let body = parse::<KnowledgeEmbeddingTestRequest>(&input, stdin, "embedding test")?;
                json!(client.knowledge_embedding_test(&body).await.map_err(anyhow::Error::msg)?)
            }
        },
        KnowledgeCommands::Dataset { command } => match command {
            DatasetCommands::List => json!(client.knowledge_datasets_list().await.map_err(anyhow::Error::msg)?),
            DatasetCommands::Create { input } => {
                let body = parse::<KnowledgeDatasetCreateRequest>(&input, stdin, "dataset create")?;
                json!(client.knowledge_dataset_create(&body).await.map_err(anyhow::Error::msg)?)
            }
            DatasetCommands::Detail { dataset_id } => json!(client.knowledge_dataset_get(&dataset_id).await.map_err(anyhow::Error::msg)?),
            DatasetCommands::Delete { dataset_id } => json!(client.knowledge_dataset_delete(&dataset_id).await.map_err(anyhow::Error::msg)?),
        },
        KnowledgeCommands::Document { command } => match command {
            DocumentCommands::List { dataset_id, status } => json!(client.knowledge_documents_query(&dataset_id, &KnowledgeDocumentListQuery { status: status.map(Into::into) }).await.map_err(anyhow::Error::msg)?),
            DocumentCommands::Create { dataset_id, input } => {
                let body = parse::<KnowledgeDocumentCreateRequest>(&input, stdin, "document create")?;
                json!(client.knowledge_document_create(&dataset_id, &body).await.map_err(anyhow::Error::msg)?)
            }
            DocumentCommands::Import { dataset_id, input } => {
                let body = parse::<KnowledgeDocumentImportRequest>(&input, stdin, "document import")?;
                json!(client.knowledge_document_import(&dataset_id, &body).await.map_err(anyhow::Error::msg)?)
            }
            DocumentCommands::ImportAsync { dataset_id, input } => {
                let body = parse::<KnowledgeDocumentImportRequest>(&input, stdin, "document async import")?;
                json!(client.knowledge_document_import_async(&dataset_id, &body).await.map_err(anyhow::Error::msg)?)
            }
            DocumentCommands::Batch { input } => {
                let body = parse::<KnowledgeDocumentsBatchActionRequest>(&input, stdin, "document batch")?;
                json!(client.knowledge_documents_batch_action(&body).await.map_err(anyhow::Error::msg)?)
            }
            DocumentCommands::Update { document_id, input } => {
                let body = parse::<KnowledgeDocumentPatchRequest>(&input, stdin, "document update")?;
                json!(client.knowledge_document_update(&document_id, &body).await.map_err(anyhow::Error::msg)?)
            }
            DocumentCommands::Delete { document_id } => json!(client.knowledge_document_delete(&document_id).await.map_err(anyhow::Error::msg)?),
            DocumentCommands::Chunks { document_id } => json!(client.knowledge_chunks_list(&document_id).await.map_err(anyhow::Error::msg)?),
            DocumentCommands::Reindex { document_id, input } => {
                let body = parse::<KnowledgeDocumentReindexRequest>(&input, stdin, "document reindex")?;
                json!(client.knowledge_document_reindex(&document_id, &body).await.map_err(anyhow::Error::msg)?)
            }
        },
        KnowledgeCommands::Job { job_id } => json!(client.knowledge_index_job_get(&job_id).await.map_err(anyhow::Error::msg)?),
        KnowledgeCommands::Retrieve { input } => {
            let body = parse::<KnowledgeRetrieveRequest>(&input, stdin, "knowledge retrieve")?;
            json!(client.knowledge_retrieve(&body).await.map_err(anyhow::Error::msg)?)
        }
        KnowledgeCommands::RetrievalTest { input } => {
            let body = parse::<KnowledgeRetrieveRequest>(&input, stdin, "knowledge retrieval test")?;
            json!(client.knowledge_retrieval_test(&body).await.map_err(anyhow::Error::msg)?)
        }
    };
    output::write_json(writer, &response, context.output_format())
}

fn parse<T: serde::de::DeserializeOwned>(source: &str, stdin: &mut impl Read, operation: &str) -> Result<T> {
    serde_json::from_value(read_json_input(source, stdin)?).with_context(|| format!("invalid {operation} JSON input"))
}

fn required(value: &str) -> Result<String, String> {
    let value = value.trim();
    if value.is_empty() { return Err("value must not be empty".to_string()); }
    Ok(value.to_string())
}
