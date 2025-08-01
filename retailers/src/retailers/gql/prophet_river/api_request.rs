pub(super) const PAGINATION_REPLACEMENT_KEY: &str = "{{pagination_token}}";

pub(super) const API_QUERY_REQUEST: &str = r#"
{
	site {
		products(
			hideOutOfStock: true
			{{pagination_token}}
			first: 50
    ) {
		pageInfo {
			endCursor
			hasNextPage
		}
		edges {
			node {
				categories {
					edges {
						node {
							breadcrumbs(depth: 99) {
								edges {
									node {
										entityId
										name
										path
									}
								}
							}
						}
					}
				}
				name
				inventory {
					isInStock
					hasVariantInventory
				}
				path
				defaultImage {
					url(width: 800)
				}
				prices(currencyCode: CAD) {
					price {
						value
					}
					salePrice {
						value
					}
				}
			}
		}
	}}
}
"#;
