use rust_bert::pipelines::sentence_embeddings::SentenceEmbeddingsModel;
use rust_bert::pipelines::ner::NERModel;

pub fn build_knowledge_graph(
    text: &str,
    bert_model: &SentenceEmbeddingsModel,
    ner_model: &NERModel,
) -> anyhow::Result<Vec<(String, Vec<f32>)>> {
    let entities_res = ner_model.predict(&[text]);
    
    let entities: Result<Vec<_>, _> = entities_res
        .first()
        .ok_or_else(|| anyhow::anyhow!("Failed to get NER results"))?
        .iter()
        .map(|entity| {
            let entity_text = entity.word.clone();
            let embedding = bert_model.encode(&[&entity_text])?
                .into_iter()
                .next()
                .ok_or_else(|| anyhow::anyhow!("Failed to get embedding for entity: {}", entity_text))?;
            Ok((entity_text, embedding))
        })
        .collect();
        
    entities
}