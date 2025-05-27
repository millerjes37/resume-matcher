// src/tfidf.rs
use std::collections::HashMap;

pub struct TfIdf {
    documents: Vec<String>,
    vocab: HashMap<String, usize>,
    idf: HashMap<String, f32>,
}

pub struct TfIdfBuilder {
    documents: Vec<String>,
}

impl TfIdfBuilder {
    pub fn new() -> Self {
        Self {
            documents: Vec::new(),
        }
    }

    pub fn add(&mut self, document: &str) {
        self.documents.push(document.to_lowercase());
    }

    pub fn build(self) -> TfIdf {
        let mut vocab = HashMap::new();
        let mut doc_count = HashMap::new();
        
        // Build vocabulary and count document frequency
        for (doc_idx, doc) in self.documents.iter().enumerate() {
            let words: Vec<&str> = doc.split_whitespace().collect();
            let mut seen_words = std::collections::HashSet::new();
            
            for word in words {
                let word = word.trim_matches(|c: char| !c.is_alphanumeric()).to_lowercase();
                if !word.is_empty() {
                    if !vocab.contains_key(&word) {
                        vocab.insert(word.clone(), vocab.len());
                    }
                    
                    if seen_words.insert(word.clone()) {
                        *doc_count.entry(word).or_insert(0) += 1;
                    }
                }
            }
        }

        // Calculate IDF
        let total_docs = self.documents.len() as f32;
        let idf: HashMap<String, f32> = doc_count
            .into_iter()
            .map(|(word, count)| {
                let idf_score = (total_docs / count as f32).ln();
                (word, idf_score)
            })
            .collect();

        TfIdf {
            documents: self.documents,
            vocab,
            idf,
        }
    }
}

impl TfIdf {
    pub fn tf_idf(&self, term: &str, document: &str) -> Option<f32> {
        let term = term.to_lowercase();
        let document = document.to_lowercase();
        
        // Calculate TF
        let words: Vec<&str> = document.split_whitespace().collect();
        let term_count = words.iter()
            .filter(|&&word| {
                let clean_word = word.trim_matches(|c: char| !c.is_alphanumeric()).to_lowercase();
                clean_word == term
            })
            .count() as f32;
        
        if term_count == 0.0 {
            return Some(0.0);
        }
        
        let tf = term_count / words.len() as f32;
        
        // Get IDF
        let idf = self.idf.get(&term).copied().unwrap_or(0.0);
        
        Some(tf * idf)
    }
}