//! Time conversion traits and functions

#[cfg(not(feature = "std"))]
use crate::format;
use crate::{google::protobuf::Timestamp, Error};
pub trait ToMillis {
    /// Convert protobuf timestamp into milliseconds since epoch

    /// Note there is a resolution difference, as timestamp uses nanoseconds
    ///
    /// # Arguments
    ///
    /// * millis - time since epoch, in milliseconds
    ///
    /// # Panics
    ///  
    /// Panics when timestamp doesn't fit `u64` type
    fn to_millis(&self) -> Result<u64, Error>;

    #[deprecated(note = "use `to_millis` instead", since = "1.3.1")]
    fn to_milis(&self) -> u64 {
        self.to_millis()
            .expect("cannot convert time to milliseconds")
    }
}

impl ToMillis for Timestamp {
    /// Convert protobuf timestamp into milliseconds since epoch
    fn to_millis(&self) -> Result<u64, Error> {
        chrono::DateTime::from_timestamp(self.seconds, self.nanos as u32)
            .map(|t| t.to_utc().timestamp_millis())
            .and_then(|t| t.try_into().ok())
            .ok_or(Error::time_conversion(format!(
                "time value {:?} out of range",
                self
            )))
    }
}

pub trait FromMillis: Sized {
    /// Create protobuf timestamp from milliseconds since epoch
    ///
    /// Note there is a resolution difference, as timestamp uses nanoseconds
    ///
    /// # Arguments
    ///
    /// * millis - time since epoch, in milliseconds; must fit `i64` type
    fn from_millis(millis: u64) -> Result<Self, Error>;
    #[deprecated(note = "use `from_millis` instead", since = "1.3.1")]
    fn from_milis(millis: u64) -> Self
    where
        Self: Sized,
    {
        Self::from_millis(millis).expect("conversion from milliseconds should not fail")
    }
}

impl FromMillis for Timestamp {
    /// Create protobuf timestamp from milliseconds since epoch
    ///
    /// Note there is a resolution difference, as timestamp uses nanoseconds
    ///
    /// # Panics
    ///  
    /// Panics when `millis` don't fit `i64` type
    fn from_millis(millis: u64) -> Result<Self, Error> {
        let ts_millis = millis
            .try_into()
            .map_err(|e| Error::time_conversion(format!("milliseconds out of range: {:?}", e)))?;

        let dt = chrono::DateTime::from_timestamp_millis(ts_millis)
            .ok_or(Error::time_conversion(format!(
                "cannot create date/time from milliseconds {}",
                ts_millis,
            )))?
            .to_utc();

        Ok(Self {
            nanos: dt.timestamp_subsec_nanos() as i32,
            seconds: dt.timestamp(),
        })
    }
}
