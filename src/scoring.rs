use crate::{ResumeLineData, compute_cosine_similarity};
use crate::tfidf::TfIdf;

pub fn score_lines(
    lines: &[ResumeLineData],
    jd_sentence_embeddings: &[Vec<f32>],
    tfidf_model: &TfIdf,
    jd_entities: &[(String, Vec<f32>)],
    jd_text: &str,
) -> Vec<(String, f32)> {
    let mut scored_lines = Vec::new();
    
    for line_data in lines {
        // Semantic similarity - find the maximum similarity with any JD sentence
        let semantic_score = jd_sentence_embeddings
            .iter()
            .map(|jd_emb| compute_cosine_similarity(&line_data.embedding, jd_emb))
            .fold(0.0, f32::max);

        // TF-IDF score - sum up TF-IDF scores for all words in the resume line
        let words: Vec<&str> = line_data.line.split_whitespace().collect();
        let sentences: Vec<&str> = jd_text
            .split('.')
            .filter(|s| !s.trim().is_empty())
            .collect();
            
        let tfidf_score: f32 = words
            .iter()
            .map(|word| {
                sentences
                    .iter()
                    .map(|sentence| tfidf_model.tf_idf(word, sentence).unwrap_or(0.0))
                    .fold(0.0, f32::max)
            })
            .sum();

        // Graph similarity - average similarity between resume line entities and JD entities
        let graph_score = if !line_data.entities.is_empty() && !jd_entities.is_empty() {
            let similarities: Vec<f32> = line_data
                .entities
                .iter()
                .flat_map(|(_, line_entity_embedding)| {
                    jd_entities
                        .iter()
                        .map(|(_, jd_entity_embedding)| {
                            compute_cosine_similarity(line_entity_embedding, jd_entity_embedding)
                        })
                })
                .collect();
                
            if similarities.is_empty() {
                0.0
            } else {
                similarities.iter().sum::<f32>() / similarities.len() as f32
            }
        } else {
            0.0
        };

        // Weighted combination of scores
        let total_score = 0.4 * semantic_score + 0.3 * tfidf_score + 0.3 * graph_score;
        scored_lines.push((line_data.line.clone(), total_score));
    }
    
    scored_lines
}