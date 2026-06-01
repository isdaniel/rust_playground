use elasticsearch::http::transport::Transport;
use elasticsearch::{BulkParts, DeleteParts, Elasticsearch, IndexParts, SearchParts};
use elasticsearch::indices::IndicesCreateParts;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::models::Product;

pub fn create_client(url: &str) -> Result<Elasticsearch, elasticsearch::Error> {
    let transport = Transport::single_node(url)?;
    Ok(Elasticsearch::new(transport))
}

pub async fn init_index(client: &Elasticsearch) -> Result<(), Box<dyn std::error::Error>> {
    let index_body = json!({
        "settings": {
            "number_of_shards": 1,
            "number_of_replicas": 0,
            "max_ngram_diff": 10,
            "analysis": {
                "filter": {
                    "edge_ngram_filter": {
                        "type": "edge_ngram",
                        "min_gram": 2,
                        "max_gram": 10
                    }
                },
                "analyzer": {
                    "autocomplete_analyzer": {
                        "type": "custom",
                        "tokenizer": "standard",
                        "filter": ["lowercase", "edge_ngram_filter"]
                    },
                    "search_analyzer": {
                        "type": "custom",
                        "tokenizer": "standard",
                        "filter": ["lowercase"]
                    }
                }
            }
        },
        "mappings": {
            "properties": {
                "id":          { "type": "keyword" },
                "name":        { "type": "text", "analyzer": "autocomplete_analyzer", "search_analyzer": "search_analyzer" },
                "description": { "type": "text", "analyzer": "standard" },
                "category":    { "type": "keyword" },
                "price":       { "type": "double" }
            }
        }
    });

    let response = client
        .indices()
        .create(IndicesCreateParts::Index("products"))
        .body(index_body)
        .send()
        .await?;

    if response.status_code().is_success() {
        log::info!("Elasticsearch index 'products' created");
    } else {
        let body = response.json::<Value>().await?;
        let already_exists = body["error"]["type"]
            .as_str()
            .map(|t| t == "resource_already_exists_exception")
            .unwrap_or(false);

        if already_exists {
            log::info!("Elasticsearch index 'products' already exists");
        } else {
            log::error!("Failed to create ES index: {:?}", body);
            return Err(format!("Failed to create ES index: {}", body).into());
        }
    }

    Ok(())
}

pub async fn index_product(
    client: &Elasticsearch,
    product: &Product,
) -> Result<(), Box<dyn std::error::Error>> {
    let doc = json!({
        "id": product.id.to_string(),
        "name": product.name,
        "description": product.description,
        "category": product.category,
        "price": product.price
    });

    let response = client
        .index(IndexParts::IndexId("products", &product.id.to_string()))
        .body(doc)
        .send()
        .await?;

    if !response.status_code().is_success() {
        let body = response.json::<Value>().await?;
        log::error!("Failed to index product {}: {:?}", product.id, body);
    }

    Ok(())
}

pub async fn delete_product(
    client: &Elasticsearch,
    id: Uuid,
) -> Result<(), Box<dyn std::error::Error>> {
    let response = client
        .delete(DeleteParts::IndexId("products", &id.to_string()))
        .send()
        .await?;

    if !response.status_code().is_success() {
        let body = response.json::<Value>().await?;
        log::warn!("Failed to delete product {} from ES: {:?}", id, body);
    }

    Ok(())
}

pub async fn search_products(
    client: &Elasticsearch,
    query: &str,
) -> Result<Vec<(Uuid, f64)>, Box<dyn std::error::Error>> {
    let search_body = json!({
        "query": {
            "bool": {
                "should": [
                    {
                        "multi_match": {
                            "query": query,
                            "fields": ["name^3", "description"],
                            "type": "best_fields",
                            "fuzziness": "AUTO"
                        }
                    },
                    {
                        "match_phrase_prefix": {
                            "name": {
                                "query": query,
                                "boost": 2.0
                            }
                        }
                    },
                    {
                        "match": {
                            "name": {
                                "query": query,
                                "analyzer": "search_analyzer",
                                "boost": 1.5
                            }
                        }
                    }
                ],
                "minimum_should_match": 1
            }
        },
        "size": 50,
        "_source": ["id"]
    });

    let response = client
        .search(SearchParts::Index(&["products"]))
        .body(search_body)
        .send()
        .await?;

    let body = response.json::<Value>().await?;
    let hits = body["hits"]["hits"]
        .as_array()
        .cloned()
        .unwrap_or_default();

    let results: Vec<(Uuid, f64)> = hits
        .iter()
        .filter_map(|hit| {
            let id_str = hit["_source"]["id"].as_str()?;
            let id = Uuid::parse_str(id_str).ok()?;
            let score = hit["_score"].as_f64().unwrap_or(0.0);
            Some((id, score))
        })
        .collect();

    Ok(results)
}

pub async fn bulk_index_products(
    client: &Elasticsearch,
    products: &[Product],
) -> Result<(), Box<dyn std::error::Error>> {
    if products.is_empty() {
        return Ok(());
    }

    let mut body: Vec<Value> = Vec::with_capacity(products.len() * 2);

    for product in products {
        body.push(json!({
            "index": {
                "_index": "products",
                "_id": product.id.to_string()
            }
        }));
        body.push(json!({
            "id": product.id.to_string(),
            "name": product.name,
            "description": product.description,
            "category": product.category,
            "price": product.price
        }));
    }

    let bulk_body: Vec<bytes::Bytes> = body
        .iter()
        .map(|v| bytes::Bytes::from(serde_json::to_vec(v).unwrap()))
        .collect();

    let response = client
        .bulk(BulkParts::None)
        .body(bulk_body)
        .send()
        .await?;

    if !response.status_code().is_success() {
        let resp_body = response.json::<Value>().await?;
        log::error!("Bulk index failed: {:?}", resp_body);
    } else {
        log::info!("Bulk indexed {} products into Elasticsearch", products.len());
    }

    Ok(())
}
