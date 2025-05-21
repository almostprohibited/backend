use std::time::Duration;

use async_trait::async_trait;
use crawler::{request::Request, traits::Crawler, unprotected::UnprotectedCrawler};
use serde_json::Value;
use tokio::time::sleep;
use tracing::{debug, trace};

use crate::{
    errors::RetailerError,
    results::{
        constants::{ActionType, AmmunitionType, FirearmClass, FirearmType},
        firearm::FirearmResult,
    },
};

#[async_trait]
pub trait Retailer {
    // abstract methods
    async fn build_page_request(
        &self,
        page_num: u64,
        search_param: &SearchParams,
    ) -> Result<Request, RetailerError>;
    async fn parse_response(
        &self,
        response: &String,
        search_param: &SearchParams,
    ) -> Result<Vec<FirearmResult>, RetailerError>;
    fn get_search_parameters(&self) -> Result<Vec<SearchParams>, RetailerError>;
    fn get_num_pages(&self, response: &String) -> Result<u64, RetailerError>;
    fn get_crawler(&self) -> UnprotectedCrawler;
    fn get_page_cooldown(&self) -> u64;

    // implemented methods
    async fn get_firearms(&self) -> Result<Vec<FirearmResult>, RetailerError> {
        let mut firearms: Vec<FirearmResult> = Vec::new();

        for search_param in self.get_search_parameters()? {
            let mut page: u64 = 0;
            let mut max_page: u64 = 1;

            while page < max_page {
                let request = self.build_page_request(page, &search_param).await?;

                let result = self.send_request(self.get_crawler(), request).await?;

                trace!("{:?}", result);

                // commit a sin and attempt to change the loop conditions mid loop iteration
                if max_page == 1 {
                    let pages = self.get_num_pages(&result)?;
                    max_page = pages;

                    debug!(
                        "Changing max pages for '{:?}' to {}",
                        search_param.lookup, max_page
                    );
                }

                let mut page_firearms = self.parse_response(&result, &search_param).await?;
                firearms.append(&mut page_firearms);

                page = page + 1;

                sleep(Duration::from_secs(self.get_page_cooldown())).await;
            }

            break;
            sleep(Duration::from_secs(1)).await;
        }

        Ok(firearms)
    }

    async fn send_request(
        &self,
        crawler: UnprotectedCrawler,
        request: Request,
    ) -> Result<String, RetailerError> {
        Ok(crawler.make_web_request(request).await?)
    }
}

pub struct SearchParams<'a> {
    pub lookup: &'a str,
    pub action_type: Option<ActionType>,
    pub ammo_type: Option<AmmunitionType>,
    pub firearm_class: Option<FirearmClass>,
    pub firearm_type: Option<FirearmType>,
}
