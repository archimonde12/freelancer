#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod freelancer {
    use ink::prelude::string::String;
    use ink::prelude::vec::Vec;
    use ink::storage::Mapping;
    pub type TJobId = i32;

    #[derive(scale::Decode, scale::Encode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub enum JobStatus {
        OPEN,
        DOING,
        REVIEW,
        REOPEN,
        FINISH,
    }

    #[derive(scale::Decode, scale::Encode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub struct Job {
        id: TJobId,
        name: String,
        description: String,
        status: JobStatus,
        budget: u128,
        expired_at: Timestamp,
    }

    /// Defines the storage of your contract.
    /// Add new fields to the below struct in order
    /// to add new static storage fields to your contract.
    #[ink(storage)]
    #[derive(Default)]
    pub struct Freelancer {
        /// Stores a single `bool` value on the storage.
        value: bool,
        count_job: TJobId,
        job: Mapping<TJobId, Job>,
    }

    impl Freelancer {
        /// Constructor that initializes the `bool` value to `false`.
        ///
        /// Constructors can delegate to other constructors.
        #[ink(constructor)]
        pub fn new() -> Self {
            Self::default()
        }

        /// A message that can be called on instantiated contracts.
        /// This one flips the value of the stored `bool` from `true`
        /// to `false` and vice versa.
        #[ink(message)]
        #[ink(payable)]
        pub fn set(
            &mut self,
            name: Vec<u8>,
            description: Vec<u8>,
            expired_after: Timestamp,
        ) -> Job {
            let name_convert = unsafe { String::from_utf8_unchecked(name) };
            let description_convert = unsafe { String::from_utf8_unchecked(description) };
            let new_job = Job {
                id: self.count_job,
                name: name_convert,
                description: description_convert,
                status: JobStatus::OPEN,
                budget: self.env().transferred_value(),
                expired_at: self.env().block_timestamp() + expired_after,
            };
            self.count_job = self.count_job + 1;
            new_job
        }

        /// Simply returns the current value of our `bool`.
        #[ink(message)]
        pub fn get(&self, job_id: TJobId) -> Option<Job> {
            self.job.get(job_id)
        }
    }
}
