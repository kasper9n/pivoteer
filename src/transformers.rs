use csv_pipeline::{Error, Headers, Row, Transform, Transformer};
use std::collections::hash_map::DefaultHasher;
use std::fmt::Display;
use std::hash::Hash;
use std::ops::AddAssign;
use std::str::FromStr;

pub trait SumOrKeepTransform {
	/// Sum the values from this column, unless the value equals `keep`
	fn sum_but_keep<'a, N>(self, init: N, keep: &str) -> Box<dyn Transform + 'a>
	where
		N: Display + AddAssign + FromStr + Clone + 'a;
}

impl SumOrKeepTransform for Transformer {
	fn sum_but_keep<'a, N>(self, init: N, keep: &str) -> Box<dyn Transform + 'a>
	where
		N: Display + AddAssign + FromStr + Clone + 'a,
	{
		Box::new(SumOrKeep {
			name: self.name,
			from_col: self.from_col,
			value: init,
			keep: keep.to_string(),
		})
	}
}

struct SumOrKeep<N> {
	name: String,
	from_col: String,
	value: N,
	keep: String,
}
impl<V> Transform for SumOrKeep<V>
where
	V: Display + AddAssign + FromStr + Clone,
{
	fn hash(&self, hasher: &mut DefaultHasher, headers: &Headers, row: &Row) -> Result<(), Error> {
		let field = headers
			.get_field(row, &self.from_col)
			.ok_or(Error::MissingColumn(self.from_col.clone()))?;
		if field == self.keep {
			field.hash(hasher);
		}
		Ok(())
	}
	fn add_row(&mut self, headers: &Headers, row: &Row) -> Result<(), Error> {
		let field = headers
			.get_field(row, &self.from_col)
			.ok_or(Error::MissingColumn(self.from_col.clone()))?;
		if field != self.keep {
			match field.parse() {
				Ok(v) => self.value += v,
				Err(_) => return Err(Error::InvalidField(field.into())),
			};
		}
		Ok(())
	}

	fn value(&self) -> String {
		self.value.to_string()
	}
	fn name(&self) -> String {
		self.name.clone()
	}
}
