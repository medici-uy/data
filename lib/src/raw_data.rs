use std::path::PathBuf;

use anyhow::Result;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{CourseData, CourseEvaluationData, QuestionData, QuestionOptionData};

#[derive(Serialize, Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct RawCourseData {
    pub name: String,
    pub short_name: String,
    pub aliases: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<PathBuf>,
    pub year: Option<i16>,
    pub questions: Vec<RawQuestionData>,
    pub evaluations: Vec<RawCourseEvaluationData>,
}

impl RawCourseData {
    pub fn from_slice(raw_data: &[u8]) -> Result<Self> {
        Ok(serde_json::from_slice(&raw_data)?)
    }
}

impl From<CourseData> for RawCourseData {
    fn from(data: CourseData) -> Self {
        let raw_questions = data.questions.into_iter().map(Into::into).collect();
        let raw_evaluations = data.evaluations.into_iter().map(Into::into).collect();

        Self {
            name: data.name,
            short_name: data.short_name,
            aliases: data.aliases,
            image: data.image_file_name,
            year: data.year,
            questions: raw_questions,
            evaluations: raw_evaluations,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct RawQuestionData {
    pub id: Option<Uuid>,

    pub evaluation: String,
    pub source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub asked_at: Option<NaiveDate>,
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<PathBuf>,
    pub options: Vec<RawQuestionOptionData>,
}

impl From<QuestionData> for RawQuestionData {
    fn from(data: QuestionData) -> Self {
        let raw_question_options = data.question_options.into_iter().map(Into::into).collect();

        Self {
            id: Some(data.id),
            text: data.text,
            image: data.image_file_name,
            options: raw_question_options,
            evaluation: data.evaluation,
            asked_at: data.asked_at,
            source: data.source,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RawQuestionOptionData {
    pub id: Option<Uuid>,

    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub correct: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub explanation: Option<String>,
}

impl From<QuestionOptionData> for RawQuestionOptionData {
    fn from(data: QuestionOptionData) -> Self {
        Self {
            id: Some(data.id),
            text: data.text,
            correct: if data.correct {
                Some(data.correct)
            } else {
                None
            },
            explanation: data.explanation,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RawCourseEvaluationData {
    pub key: String,
    pub name: String,
}

impl From<CourseEvaluationData> for RawCourseEvaluationData {
    fn from(data: CourseEvaluationData) -> Self {
        Self {
            key: data.key,
            name: data.name,
        }
    }
}
