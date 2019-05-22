use crate::{ast::*, visitor::Visitor};
use postgres::types::{IsNull, ToSql, Type};
use rust_decimal::Decimal;
use std::{error::Error, str::FromStr};

pub struct Postgres {
    parameters: Vec<ParameterizedValue>,
}

impl Visitor for Postgres {
    const C_BACKTICK: &'static str = "\"";
    const C_WILDCARD: &'static str = "%";

    fn build<Q>(query: Q) -> (String, Vec<ParameterizedValue>)
    where
        Q: Into<Query>,
    {
        let mut postgres = Postgres {
            parameters: Vec::new(),
        };

        (
            Postgres::visit_query(&mut postgres, query.into()),
            postgres.parameters,
        )
    }

    fn add_parameter(&mut self, value: ParameterizedValue) {
        self.parameters.push(value);
    }

    fn parameter_substitution(&self) -> String {
        format!("${}", self.parameters.len())
    }

    fn visit_limit_and_offset(
        &mut self,
        limit: Option<ParameterizedValue>,
        offset: Option<ParameterizedValue>,
    ) -> Option<String> {
        match (limit, offset) {
            (Some(limit), Some(offset)) => Some(format!(
                "LIMIT {} OFFSET {}",
                self.visit_parameterized(limit),
                self.visit_parameterized(offset)
            )),
            (None, Some(offset)) => Some(format!("OFFSET {}", self.visit_parameterized(offset))),
            (Some(limit), None) => Some(format!("LIMIT {}", self.visit_parameterized(limit))),
            (None, None) => None,
        }
    }

    fn visit_insert(&mut self, insert: Insert) -> String {
        let mut result = vec![String::from("INSERT")];

        result.push(format!("INTO {}", self.visit_table(insert.table, true)));

        if insert.values.is_empty() {
            result.push("DEFAULT VALUES".to_string());
        } else {
            let columns: Vec<String> = insert
                .columns
                .into_iter()
                .map(|c| self.visit_column(Column::from(c)))
                .collect();

            let values: Vec<String> = insert
                .values
                .into_iter()
                .map(|row| self.visit_row(row))
                .collect();

            result.push(format!(
                "({}) VALUES {}",
                columns.join(", "),
                values.join(", "),
            ))
        }

        match insert.on_conflict {
            Some(OnConflict::DoNothing) => result.push(String::from("ON CONFLICT DO NOTHING")),
            None => (),
        };

        if let Some(returning) = insert.returning {
            if !returning.is_empty() {
                let values = returning.into_iter().map(|r| r.into()).collect();
                result.push(format!("RETURNING {}", self.visit_columns(values)));
            }
        };

        result.join(" ")
    }

    fn visit_aggregate_to_string(&mut self, value: DatabaseValue) -> String {
        format!("array_to_string(array_agg({}), ',')", self.visit_database_value(value))
    }
}

impl ToSql for ParameterizedValue {
    fn to_sql(
        &self,
        ty: &Type,
        out: &mut Vec<u8>,
    ) -> Result<IsNull, Box<dyn Error + 'static + Send + Sync>> {
        match self {
            ParameterizedValue::Null => Ok(IsNull::Yes),
            ParameterizedValue::Integer(integer) => match *ty {
                Type::INT2 => (*integer as i16).to_sql(ty, out),
                Type::INT4 => (*integer as i32).to_sql(ty, out),
                _ => (*integer as i64).to_sql(ty, out),
            },
            ParameterizedValue::Real(float) => match *ty {
                Type::NUMERIC => {
                    let s = float.to_string();
                    Decimal::from_str(&s).unwrap().to_sql(ty, out)
                }
                _ => float.to_sql(ty, out),
            },
            ParameterizedValue::Text(string) => string.to_sql(ty, out),
            ParameterizedValue::Boolean(boo) => boo.to_sql(ty, out),
            #[cfg(feature = "json-1")]
            ParameterizedValue::Json(value) => value.to_sql(ty, out),
            #[cfg(feature = "uuid-0_7")]
            ParameterizedValue::Uuid(value) => value.to_sql(ty, out),
            #[cfg(feature = "chrono-0_4")]
            ParameterizedValue::DateTime(value) => value.naive_utc().to_sql(ty, out),
        }
    }

    fn accepts(_: &Type) -> bool {
        true // Please check later should we make this to be more restricted
    }

    fn to_sql_checked(
        &self,
        ty: &Type,
        out: &mut Vec<u8>,
    ) -> Result<IsNull, Box<dyn Error + 'static + Send + Sync>> {
        match self {
            ParameterizedValue::Null => Ok(IsNull::Yes),
            ParameterizedValue::Integer(integer) => match *ty {
                Type::INT2 => (*integer as i16).to_sql_checked(ty, out),
                Type::INT4 => (*integer as i32).to_sql_checked(ty, out),
                _ => integer.to_sql_checked(ty, out),
            },
            ParameterizedValue::Real(float) => match *ty {
                Type::NUMERIC => {
                    let s = float.to_string();
                    Decimal::from_str(&s).unwrap().to_sql(ty, out)
                }
                _ => float.to_sql(ty, out),
            },
            ParameterizedValue::Text(string) => string.to_sql_checked(ty, out),
            ParameterizedValue::Boolean(boo) => boo.to_sql_checked(ty, out),
            #[cfg(feature = "json-1")]
            ParameterizedValue::Json(value) => value.to_sql_checked(ty, out),
            #[cfg(feature = "uuid-0_7")]
            ParameterizedValue::Uuid(value) => value.to_sql_checked(ty, out),
            #[cfg(feature = "chrono-0_4")]
            ParameterizedValue::DateTime(value) => value.naive_utc().to_sql_checked(ty, out),
        }
    }
}
