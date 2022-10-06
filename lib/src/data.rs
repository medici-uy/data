use std::fs::DirEntry;
use std::path::PathBuf;
use std::{cmp::Ordering, ffi::OsStr};

use anyhow::{bail, Result};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    hashable::Hashable,
    helpers::{read_dir_entry_data, write_data},
    RawCourseData,
};
use crate::{
    raw_data::{RawQuestionData, RawQuestionOptionData},
    RawCourseEvaluationData,
};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CourseData {
    pub key: String,
    pub name: String,
    pub short_name: String,
    pub aliases: Vec<String>,
    pub year: Option<i16>,

    #[serde(skip)]
    pub questions: Vec<QuestionData>,
    pub evaluations: Vec<CourseEvaluationData>,

    pub hash: String,
}

impl CourseData {
    pub fn new(key: String, raw: RawCourseData) -> Self {
        let questions: Vec<QuestionData> = raw.questions.into_iter().map(Into::into).collect();
        let evaluations: Vec<CourseEvaluationData> =
            raw.evaluations.into_iter().map(Into::into).collect();

        let mut data = Self {
            key,
            name: raw.name,
            short_name: raw.short_name,
            aliases: raw.aliases,
            year: raw.year,
            questions,
            evaluations,
            hash: Default::default(),
        };

        data.set_hash();

        data
    }

    pub async fn load_and_write_formatted(
        dir_entry: DirEntry,
        mut images_path: PathBuf,
    ) -> Result<Self> {
        let path = dir_entry.path();
        let mut data = Self::load(path.clone(), dir_entry)?;

        data.check()?;
        data.deduplicate();
        data.sort();

        data.set_data();

        images_path.push(data.key.clone());
        data.format(images_path).await?;

        data.clone().write(path)?;

        Ok(data)
    }

    pub fn load(path: PathBuf, dir_entry: DirEntry) -> Result<Self> {
        let raw_data = read_dir_entry_data(dir_entry)?;

        let key = path
            .file_stem()
            .and_then(OsStr::to_str)
            .expect("invalid file name")
            .to_owned();
        let raw_course_data = RawCourseData::from_slice(&raw_data[..])?;

        Ok(Self::new(key, raw_course_data))
    }

    pub fn write(self, path: PathBuf) -> Result<()> {
        let raw = self.into();
        let raw_data = serde_json::to_string_pretty::<RawCourseData>(&raw)?;

        write_data(path, raw_data)
    }

    fn sort(&mut self) {
        self.questions
            .sort_by(|a, b| match a.evaluation.cmp(&b.evaluation) {
                Ordering::Equal => match a.asked_at.cmp(&b.asked_at) {
                    Ordering::Equal => match a.text.cmp(&b.text) {
                        Ordering::Equal => a.id.cmp(&b.id),
                        ordering => ordering,
                    },
                    ordering => ordering,
                },
                ordering => ordering,
            });

        for question in self.questions.iter_mut() {
            question.sort_options();
        }
    }

    fn deduplicate(&mut self) {
        self.questions.dedup_by(|a, b| a.eq_data(b));

        for question in self.questions.iter_mut() {
            question.deduplicate_options();
        }
    }

    fn check(&self) -> Result<()> {
        for question in &self.questions {
            question.check()?;
        }

        Ok(())
    }

    fn set_data(&mut self) {
        for question in &mut self.questions {
            question.set_data(self.key.clone());
        }

        for evaluation in &mut self.evaluations {
            evaluation.set_course_key(self.key.clone());
        }
    }

    async fn format(&mut self, images_path: PathBuf) -> Result<()> {
        for question in &mut self.questions {
            question.format(images_path.clone()).await?;
        }

        Ok(())
    }
}

impl Hashable for CourseData {
    fn hashable_data(&self) -> Vec<u8> {
        let mut bytes = vec![];

        bytes.extend(self.key.as_bytes());
        bytes.extend(self.name.as_bytes());
        bytes.extend(self.short_name.as_bytes());
        bytes.extend(self.aliases.join("").as_bytes());

        if let Some(year) = self.year {
            bytes.extend(&year.to_be_bytes());
        }

        bytes.extend(
            self.questions
                .iter()
                .flat_map(|question| question.hash.as_bytes()),
        );
        bytes.extend(
            self.evaluations
                .iter()
                .flat_map(|evaluation| evaluation.hash.as_bytes()),
        );

        bytes
    }

    fn set_hash(&mut self) {
        self.hash = self.hash_data();
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct QuestionData {
    pub id: Uuid,

    pub course_key: Option<String>,
    pub evaluation: String,
    pub source: String,
    pub asked_at: Option<NaiveDate>,
    pub text: String,
    pub image_file_name: Option<PathBuf>,
    #[serde(skip)]
    pub question_options: Vec<QuestionOptionData>,

    pub hash: String,
}

impl QuestionData {
    fn new(
        id: Uuid,
        text: String,
        image_file_name: Option<PathBuf>,
        question_options: Vec<QuestionOptionData>,
        evaluation: String,
        source: String,
        asked_at: Option<NaiveDate>,
    ) -> Self {
        let mut data = Self {
            id,
            course_key: None,
            evaluation,
            source,
            asked_at,
            text,
            image_file_name: image_file_name,
            question_options,
            hash: Default::default(),
        };

        data.set_hash();

        data
    }

    fn sort_options(&mut self) {
        self.question_options.sort_by(|a, b| {
            if a.correct {
                Ordering::Less
            } else if b.correct {
                Ordering::Greater
            } else {
                a.text.cmp(&b.text)
            }
        })
    }

    fn deduplicate_options(&mut self) {
        self.question_options.dedup_by(|a, b| a.eq_data(b));
    }

    fn eq_data(&self, other: &Self) -> bool {
        self.text == other.text
            && self.evaluation == other.evaluation
            && self.question_options.len() == other.question_options.len()
            && self
                .question_options
                .iter()
                .all(|a| other.question_options.iter().any(|b| a.eq_data(b)))
    }

    fn check(&self) -> Result<()> {
        if self.question_options.len() < 2 || self.question_options.len() > 5 {
            bail!(
                "Question {} has {} option(s)",
                self.id,
                self.question_options.len()
            );
        }

        let correct_count = self
            .question_options
            .iter()
            .filter(|option| option.correct)
            .count();

        if correct_count != 1 {
            bail!("Question {} has {correct_count} correct options", self.id)
        }

        Ok(())
    }

    async fn format(&mut self, images_path: PathBuf) -> Result<()> {
        self.text = self.text.trim().into();

        if let Some(image_file_name) = &self.image_file_name {
            let stem = image_file_name.file_stem().and_then(OsStr::to_str).unwrap();

            if stem != &self.id.to_string() {
                let extension = image_file_name
                    .extension()
                    .and_then(OsStr::to_str)
                    .expect("no extension in image file name");

                let mut new_file_name = PathBuf::from(self.id.to_string());
                new_file_name.set_extension(extension);

                let mut old_path = images_path.clone();
                old_path.push(image_file_name);
                let mut new_path = images_path.clone();
                new_path.push(new_file_name.clone());
                tokio::fs::rename(old_path, new_path).await?;

                self.image_file_name.replace(new_file_name);
            }
        }

        for question_option in self.question_options.iter_mut() {
            question_option.format();
        }

        Ok(())
    }

    fn set_data(&mut self, course_key: String) {
        self.set_course_key(course_key);

        for question_option in self.question_options.iter_mut() {
            question_option.set_question_id(self.id);
        }
    }

    pub fn set_course_key(&mut self, course_key: String) {
        self.course_key.replace(course_key);
    }

    pub fn full_evaluation_key(&self) -> String {
        CourseEvaluationData::do_full_key(
            self.course_key.as_ref().expect("course key not set"),
            &self.evaluation,
        )
    }

    pub fn full_image_path(&self) -> Option<String> {
        Some(format!(
            "{}/{}",
            self.course_key.as_ref().expect("course key not set"),
            self.image_file_name.as_ref()?.as_os_str().to_string_lossy()
        ))
    }
}

impl Hashable for QuestionData {
    fn hashable_data(&self) -> Vec<u8> {
        let mut bytes = vec![];

        bytes.extend(self.id.as_bytes());
        bytes.extend(self.text.as_bytes());

        if let Some(image_file_name) = &self.image_file_name {
            bytes.extend(image_file_name.to_string_lossy().as_bytes());
        }

        bytes.extend(
            self.question_options
                .iter()
                .flat_map(|question_option| question_option.hash.as_bytes()),
        );

        bytes.extend(self.evaluation.as_bytes());
        bytes.extend(self.source.as_bytes());

        if let Some(asked_at) = self.asked_at {
            bytes.extend(asked_at.to_string().as_bytes());
        }

        bytes
    }

    fn set_hash(&mut self) {
        self.hash = self.hash_data();
    }
}

impl From<RawQuestionData> for QuestionData {
    fn from(raw: RawQuestionData) -> Self {
        let options = raw.options.into_iter().map(Into::into).collect();

        Self::new(
            raw.id.unwrap_or_else(|| Uuid::new_v4()),
            raw.text,
            raw.image,
            options,
            raw.evaluation,
            raw.source,
            raw.asked_at,
        )
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct QuestionOptionData {
    pub id: Uuid,

    pub question_id: Option<Uuid>,
    pub text: String,
    pub correct: bool,
    pub explanation: Option<String>,

    pub hash: String,
}

impl QuestionOptionData {
    fn new(id: Uuid, text: String, correct: bool, explanation: Option<String>) -> Self {
        let mut data = Self {
            id,
            question_id: None,
            text,
            correct,
            explanation,
            hash: Default::default(),
        };

        data.set_hash();

        data
    }

    fn eq_data(&self, other: &Self) -> bool {
        self.text == other.text
            && self.correct == other.correct
            && self.explanation == other.explanation
    }

    fn format(&mut self) {
        self.text = self.text.trim().into();
    }

    pub fn set_question_id(&mut self, question_id: Uuid) {
        self.question_id = Some(question_id);
    }
}

impl Hashable for QuestionOptionData {
    fn hashable_data(&self) -> Vec<u8> {
        let mut bytes = vec![];

        bytes.extend(self.id.as_bytes());
        bytes.extend(self.text.as_bytes());
        bytes.extend(&[self.correct as u8]);

        if let Some(explanation) = &self.explanation {
            bytes.extend(explanation.as_bytes());
        }

        bytes
    }

    fn set_hash(&mut self) {
        self.hash = self.hash_data();
    }
}

impl From<RawQuestionOptionData> for QuestionOptionData {
    fn from(raw: RawQuestionOptionData) -> Self {
        Self::new(
            raw.id.unwrap_or_else(|| Uuid::new_v4()),
            raw.text,
            raw.correct.unwrap_or(false),
            raw.explanation,
        )
    }
}

#[derive(Serialize, Deserialize, PartialEq, Hash, Eq, Clone, Debug)]
pub struct CourseEvaluationData {
    pub course_key: Option<String>,
    pub key: String,
    pub name: String,
    pub hash: String,
}

impl CourseEvaluationData {
    pub fn new(raw: RawCourseEvaluationData) -> Self {
        let mut data = Self {
            course_key: None,
            key: raw.key,
            name: raw.name,
            hash: Default::default(),
        };

        data.set_hash();

        data
    }

    pub fn set_course_key(&mut self, course_key: String) {
        self.course_key = Some(course_key.clone());
    }

    pub fn full_key(&self) -> String {
        Self::do_full_key(
            self.course_key.as_ref().expect("course key not set"),
            &self.key,
        )
    }

    pub fn do_full_key(course_key: &str, key: &str) -> String {
        format!("{}{COURSE_EVALUATION_KEY_SEPARATOR}{}", course_key, key)
    }
}

impl Hashable for CourseEvaluationData {
    fn hashable_data(&self) -> Vec<u8> {
        let mut bytes = vec![];

        bytes.extend(self.name.as_bytes());

        bytes
    }

    fn set_hash(&mut self) {
        self.hash = self.hash_data();
    }
}

impl From<RawCourseEvaluationData> for CourseEvaluationData {
    fn from(raw: RawCourseEvaluationData) -> Self {
        Self::new(raw)
    }
}

pub const COURSE_EVALUATION_KEY_SEPARATOR: &str = "/";
