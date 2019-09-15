use rstd::vec::Vec;
use parity_codec::{Decode, Encode};
use support::{
    decl_event, decl_module, decl_storage, dispatch::Result, ensure, StorageMap, StorageValue,traits::{Currency,LockIdentifier,LockableCurrency,WithdrawReasons}};
use runtime_primitives::traits::Bounded;
use system::ensure_signed;
//use parity_codec::alloc::vec::Vec;

pub trait Trait: system::Trait+balances::Trait {
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
    type Currency: LockableCurrency<Self::AccountId, Moment=Self::BlockNumber>;
}

const BUY_Lock: LockIdentifier = *b"buy_lock";

decl_event! {
    pub enum Event<T>
    where
    <T as system::Trait>::AccountId
    {
       Transfered(Vec<u8>),
       Confirmed(AccountId,usize),
    }
}

#[derive(Encode, Decode, Default, Clone, PartialEq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct OrderForm<Balance> {
    id: usize,
    order_name: Vec<u8>,
    content: Vec<u8>,
    unit_price: Balance,
}

#[derive(Encode, Decode, Default, Clone, PartialEq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct DataMetadata<AccountId>{
    data_name: Vec<u8>,
    to_company: AccountId,
    order_id: usize,
    ipfs_hash: u64,
}


decl_storage! {
    trait Store for Module<T: Trait> as DataStore {

        pub Company get(get_order): map T::AccountId => Vec<OrderForm<T::Balance>>;

        People get(get_data): map T::AccountId => Vec<DataMetadata<T::AccountId>>;

        // todo: we will use IPFS to store the data. Now we cannot use IPFS on the substrate runtime.
        Data get(get_content): map u64 => Vec<u8>;

        Nonce get(get_n): u64;

    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        fn deposit_event<T>() = default;

        fn buy_data(origin, person: T::AccountId, order_id: usize) -> Result {
            let company = ensure_signed(origin)?;
            ensure!(company != person, "you can't buy your own data");
            let is_existed = <Company<T>>::exists(&company);
            if is_existed {
                Self::transfer_data(company,person,order_id)
            }else {
                Err("no company")
            }
        }

        fn confirm_data(origin, person: T::AccountId, order_id: usize) -> Result {
            let company = ensure_signed(origin)?;
            ensure!(company != person, "you can't confirm to buy your data");
            T::Currency::remove_lock(BUY_Lock,&person);
            Self::deposit_event(RawEvent::Confirmed(person,order_id));
            Ok(())
        }

        fn upload_order(origin, order_name: Vec<u8>, content: Vec<u8>, unit_price: T::Balance) -> Result{
            let company = ensure_signed(origin)?;
            let is_existed = <Company<T>>::exists(&company);
            if !is_existed {
                <Company<T>>::insert(company.clone(), Vec::new());
            }
            let mut order_list = Self::get_order(&company);
            let new_order: OrderForm<T::Balance> = OrderForm{
                id: order_list.len(),
                order_name: order_name,
                content: content,
                unit_price: unit_price,
            };
            order_list.push(new_order);
            Ok(())
        }

        fn upload_data(origin, data_name: Vec<u8>, data_content: Vec<u8>, to_company: T::AccountId, order_id: usize) -> Result {
            let person = ensure_signed(origin)?;
            let is_existed = <People<T>>::exists(&to_company);
            if !is_existed {
                <People<T>>::insert(person.clone(), Vec::new());
            }
            let mut metadata_list = Self::get_data(&person);
            let ipfs_hash = Self::add_by_ipfs(data_content);
            let new_data: DataMetadata<T::AccountId> = DataMetadata{
                ipfs_hash: ipfs_hash,
                data_name: data_name,
                to_company: to_company,
                order_id: order_id,
            };
            metadata_list.push(new_data);
            Ok(())
        }

        fn update_data(origin, data_name: Vec<u8>, data_content: Vec<u8>, to_company: T::AccountId, order_id: usize) -> Result {
            let person = ensure_signed(origin)?;
            let is_existed = <People<T>>::exists(&to_company);
            if !is_existed {
                <People<T>>::insert(person.clone(), Vec::new());
                Err("no person")
            }else {
                let metadata_list = Self::get_data(&person);
                for mut metadata in metadata_list {
                    if metadata.to_company == to_company && metadata.order_id == order_id {
                        let ipfs_hash = Self::add_by_ipfs(data_content);
                        metadata.ipfs_hash = ipfs_hash;
                        metadata.data_name = data_name;
                        break;
                    }
                };
                Ok(())
            }

        }

    }
}

//#[cfg_attr(feature = "std",derive(Debug, Serialize, Deserialize))]
//pub struct IpfsResp {
//    pub name: Vec<u8>,
//    pub hash: Vec<u8>,
//    pub size: Vec<u8>,
//}

impl<T: Trait> Module<T> {

    fn transfer_data(company: T::AccountId, person: T::AccountId, order_id: usize) -> Result{
        let orders = Self::get_order(&company);

        let metadata_list = Self::get_data(&person);
        for metadata in metadata_list {
            if metadata.to_company == company && metadata.order_id == order_id {

                for order in &orders {
                    if order.id == order_id {
                        let data = Self::get_by_ipfs(metadata.ipfs_hash);
                        let pay = order.unit_price;
                        <balances::Module<T> as Currency<_>>::transfer(&company, &person, pay)?;
                        T::Currency::set_lock(BUY_Lock,&person, Bounded::max_value(), T::BlockNumber::max_value(),WithdrawReasons::all());
                        Self::deposit_event(RawEvent::Transfered(data));
                    }
                }


            }
        };

        Ok(())
    }

    fn add_by_ipfs(value: Vec<u8>) -> u64
    {
       /*let mut opts = RequestInit::new();
       opts.method("POST");
       let req = Request::new_with_str_and_init(
           "http://localhost:5001/api/v0/add",
           &opts,
       );
       opts.body(value);
       opts.mode(RequestMode::Cors);
       req.headers().set("Content-Type","multipart/form-data");
       let window = web_sys::window().ok_or_else(|| JsValue::from_str("Could not get a window object")).unwrap();
       let resp_value = JsFuture::from(window.fetch_with_request(&req))?;
       let resp: Response = resp_value.dyn_into().unwrap();
       let json = JsFuture::from(resp.json()?)?;
       let ipfs_resp: IpfsResp = json.into_serde().unwrap();
       ipfs_resp.hash*/

        // simulate store into IPFS
        let ipfs_hash= Self::get_n();
        <Data<T>>::insert(&ipfs_hash,value);
        <Nonce<T>>::mutate(|n| *n += 1);
        ipfs_hash
    }

    fn get_by_ipfs(key: u64) -> Vec<u8> {
        /*let mut opts = RequestInit::new();
       opts.method("GET");
       opts.mode(RequestMode::Cors);
       let key_str = str::from_utf8(&key).unwrap();
       let req = Request::new_with_str_and_init(
           "http://localhost:8080/ipfs/"+key_str,
           &opts,
       );
       let window = web_sys::window().ok_or_else(|| JsValue::from_str("Could not get a window object")).unwrap();
       let resp_value = JsFuture::from(window.fetch_with_request(&req))?;
       let resp: Response = resp_value.dyn_into().unwrap();
       let txt = JsFuture::from(resp.text()?)?;
       let ipfs_value: Vec<u8> = txt.into_serde().unwrap();
       ipfs_value*/
        Self::get_content(key)

    }

}
