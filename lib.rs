#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod freelancer {
    use ink::prelude::string::String;
    use ink::prelude::vec::Vec;
    use ink::storage::Mapping;
    use scale::{Decode, Encode};
    pub type TJobId = u32;

    #[derive(scale::Decode, scale::Encode, PartialEq, Eq)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub enum JobStatus {
        Open,
        Doing,
        Review,
        Reopen,
        Finish,
    }

    #[derive(Encode, Decode, Debug, PartialEq, Eq, Copy, Clone)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        NotOwner,
        JobExpired,
        JobOwnerCanNotAcquire,
        NotJobOwner,
        NotJobAcquirer,
        JobNotExists,
        JobStatusIsNotOpen,
        JobStatusIsNotDoing,
        JobStatusIsNotReview,
        JobStatusIsNotReopen,
        JobStatusIsNotFinish,
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
        owner:AccountId,
    }

    #[derive(scale::Decode, scale::Encode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub struct ViewOpenningJobResponse {
       data:Vec<Job>,
       total:u32,
    }

    #[derive(scale::Decode, scale::Encode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub struct UserStatistic {
       earning:Balance,
       paying:Balance,
    }


    /// Defines the storage of your contract.
    /// Add new fields to the below struct in order
    /// to add new static storage fields to your contract.
    #[ink(storage)]
    pub struct Freelancer {
        owner: AccountId,
        count_job: TJobId,
        jobs: Mapping<TJobId, Job>,
        job_owners: Mapping<TJobId, AccountId>,
        job_acquirers: Mapping<TJobId, AccountId>,
        opening_jobs: Vec<TJobId>,
        fee_percent: u32,
        earnings:Mapping<AccountId,Balance>,
        payings:Mapping<AccountId,Balance>,
        system_profit:Balance,
    }

    #[ink(impl)]
    impl Freelancer {
        fn _add_new_job(
            &mut self,
            owner: AccountId,
            name: Vec<u8>,
            description: Vec<u8>,
            expired_after: Timestamp,
        ) -> Job {
            let name_convert = unsafe { String::from_utf8_unchecked(name) };
            let description_convert = unsafe { String::from_utf8_unchecked(description) };
            let budget=self._calculate_budget(self.env().transferred_value());
            let new_job = Job {
                id: self.count_job,
                name: name_convert,
                description: description_convert,
                status: JobStatus::Open,
                budget,
                expired_at: self.env().block_timestamp() + expired_after,
                owner:self.env().caller(),
            };

            self.jobs.insert(new_job.id, &new_job);
            self.job_owners.insert(new_job.id, &owner);
            self.opening_jobs.push(self.count_job);
            match self.payings.get(owner) {
                None => {self.payings.insert(owner, &self.env().transferred_value());},
                Some(paying) => {self.payings.insert(owner,&(paying+self.env().transferred_value()));}
            }

            self.count_job += 1;
            new_job
        }

        fn _ensure_job_owner(&self, job_id: TJobId) -> Result<(), Error> {
            let owner=self.env().caller();
            let _owner = self.job_owners.get(job_id);
            match _owner {
                None => return Err(Error::NotJobOwner),
                Some(real_owner) => {
                    if real_owner != owner {
                        return Err(Error::NotJobOwner);
                    };
                }
            }
            Ok(())
        }

        fn _ensure_job_acquirer(&self, job_id: TJobId) -> Result<(), Error> {
            let acquirer= self.env().caller();
            let _owner = self.job_acquirers.get(job_id);
            match _owner {
                None => return Err(Error::NotJobAcquirer),
                Some(real_owner) => {
                    if real_owner != acquirer {
                        return Err(Error::NotJobAcquirer);
                    };
                }
            }
            Ok(())
        }

        fn _ensure_not_job_owner(&self, job_id: TJobId, owner: AccountId) -> Result<(), Error> {
            let _owner = self.job_owners.get(job_id);
            match _owner {
                None => return Ok(()),
                Some(real_owner) => {
                    if real_owner == owner {
                        return Err(Error::JobOwnerCanNotAcquire);
                    }
                }
            }
            Ok(())
        }

        fn _ensure_active_job(&self, job_id: TJobId) -> Result<(), Error> {
            let job = self._get_job(job_id)?;
            if job.expired_at < self.env().block_timestamp() {
                return Err(Error::JobExpired);
            }
            Ok(())
        }

        fn _ensure_job_status(&self, job_id: TJobId, status: JobStatus) -> Result<(), Error> {
            let job = self._get_job(job_id)?;
            if job.status != status {
                match status {
                    JobStatus::Open => return Err(Error::JobStatusIsNotOpen),
                    JobStatus::Doing => return Err(Error::JobStatusIsNotDoing),
                    JobStatus::Review => return Err(Error::JobStatusIsNotReview),
                    JobStatus::Reopen => return Err(Error::JobStatusIsNotReopen),
                    JobStatus::Finish => return Err(Error::JobStatusIsNotFinish),
                };
            }
            Ok(())
        }

        fn _ensure_owner(&self) -> Result<(), Error> {
            let caller = self.env().caller();
            if caller != self.owner {
                return Err(Error::NotOwner);
            }
            Ok(())
        }

        fn _get_job(&self, job_id: TJobId) -> Result<Job, Error> {
            match self.jobs.get(job_id) {
                None => Err(Error::JobNotExists),
                Some(job) => Ok(job),
            }
        }

        fn _update_job_status(&mut self, job_id: TJobId, status: JobStatus) -> Result<(), Error> {
            let mut job = self._get_job(job_id)?;
            match status {
                JobStatus::Finish => {
                    self._payout_for(job_id)?
                }
                JobStatus::Open =>{},
                JobStatus::Doing => {
                    self.opening_jobs.retain(|&el| el!=job_id);
                },
                JobStatus::Review => {},
                JobStatus::Reopen => {},
            }
            job.status = status;
            self.jobs.insert(job_id, &job);

            Ok(())
        }

        fn _calculate_budget(&self, value: u128) -> u128 {
            value * u128::from(self.fee_percent) / 100
        }

        fn _payout_for(&mut self, job_id: TJobId) -> Result<(), Error> {
            let acquirer = self.job_acquirers.get(job_id);
            let job = self._get_job(job_id)?;
            match acquirer {
                None => return Err(Error::NotJobAcquirer),
                Some(_acquirer) => {
                    self.env()
                        .transfer(_acquirer, job.budget)
                        .unwrap_or_else(|err| panic!("transfer failed: {:?}", err));
                    match self.earnings.get(_acquirer) {
                        None => {self.earnings.insert(_acquirer, &job.budget);},
                        Some(earning) => {self.earnings.insert(_acquirer,&(earning+job.budget));}
                    }
                }
            }

            Ok(())
        }

    }

    impl Freelancer {
        #[ink(constructor)]
        pub fn new() -> Self {
            Self {
                owner: Self::env().caller(),
                count_job: Default::default(),
                jobs: Default::default(),
                job_owners: Default::default(),
                job_acquirers: Default::default(),
                opening_jobs: Default::default(),
                fee_percent: Default::default(),
                earnings:Default::default(),
                payings:Default::default(),
                system_profit:Default::default()
            }
        }
        //Contract Owner Functions
        #[ink(message)]
        pub fn update_fee_percent(&mut self,value:u32)-> Result<(),Error> {
            self._ensure_owner()?; 
            self.fee_percent=value;
            Ok(())
        }

        //Job Owner Functions
        #[ink(message)]
        #[ink(payable)]
        pub fn create_job(
            &mut self,
            name: Vec<u8>,
            description: Vec<u8>,
            expired_after: Timestamp,
        ) -> Job {
            self._add_new_job(self.env().caller(), name, description, expired_after)
        }

        #[ink(message)]
        pub fn reopen_job(&mut self, job_id: TJobId) -> Result<(), Error> {
            self._ensure_job_owner(job_id)?;
            self._ensure_job_status(job_id, JobStatus::Review)?;
            self._update_job_status(job_id, JobStatus::Reopen)?;
            Ok(())
        }

        #[ink(message)]
        pub fn finish_job(&mut self, job_id: TJobId) -> Result<(), Error> {
            self._ensure_job_owner(job_id)?;
            self._ensure_job_status(job_id, JobStatus::Review)?;
            self._update_job_status(job_id, JobStatus::Finish)?;
            Ok(())
        }

        //Freelancer Functions

        /// query available openning jobs for freelancer
        #[ink(message)]
        pub fn view_open_jobs(&self,page: u16,page_size:u16) -> ViewOpenningJobResponse {
           let  skip=page*page_size;
           let openning_jobs_count=self.opening_jobs.len();
           if usize::from(skip)>openning_jobs_count {return ViewOpenningJobResponse{data:Vec::new(),total:u32::try_from(openning_jobs_count).unwrap()}};
           let mut jobs=self.opening_jobs.clone().split_off(usize::from(skip));
           jobs.truncate(usize::from(page_size));
           ViewOpenningJobResponse{
            data: jobs.iter().map(|job| {self._get_job(*job).unwrap()}).collect(),
            total:u32::try_from(openning_jobs_count).unwrap()
           }
          
        }

        #[ink(message)]
        pub fn view_job(&self,job_id: TJobId) -> Result<Job,Error>{
            self._get_job(job_id)
        }

        #[ink(message)]
        pub fn acquire_job(&mut self, job_id: TJobId) -> Result<(), Error> {
            let caller = self.env().caller();
            self._ensure_not_job_owner(job_id, caller)?;
            self._ensure_job_status(job_id, JobStatus::Open)?;
            self._ensure_active_job(job_id)?;
            self._update_job_status(job_id, JobStatus::Doing)?;
            self.job_acquirers.insert(job_id,&caller);
            Ok(())
        }

        #[ink(message)]
        pub fn review_request(&mut self,job_id: TJobId)->Result<(),Error> {
            self._ensure_job_acquirer(job_id)?;
            let is_doing=self._ensure_job_status(job_id, JobStatus::Doing).is_ok();
            let is_reopen=self._ensure_job_status(job_id, JobStatus::Reopen).is_ok();
            if is_doing||is_reopen{
                self._update_job_status(job_id, JobStatus::Review)?;
            }
            Ok(())
        }

        #[ink(message)]
        pub fn user_statistic(&self)-> UserStatistic{
            let caller=self.env().caller();
            UserStatistic{
                earning:self.earnings.get(caller).unwrap_or(0),
                paying:self.payings.get(caller).unwrap_or(0)
            }
        }

        #[ink(message)]
        pub fn check_balance(&self)->Result<Balance,Error>{
            self._ensure_owner()?;
            Ok(self.env().balance())
        }
    }
}
