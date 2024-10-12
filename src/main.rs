#![no_std]
#![no_main]

use core::panic::PanicInfo;
use core::alloc::{GlobalAlloc, Layout};
use core::ptr::{null_mut, NonNull};

//interrupt::free(|_| { /* code qui va pas s'interrompre */})

#[panic_handler]
fn panic(_panic: &PanicInfo) -> ! {
    loop {}
}

// Création de la structure de mon allocateur (modèle: free list allocator)
struct MyAlloc {
    free_list: Option<NonNull<FreeBlock>>,
    pool_start: *mut u8, 
    pool_size: usize,
}

struct FreeBlock {
    size: usize,
    next: Option<NonNull<FreeBlock>>, // Contient le prochain block libre
}

impl MyAlloc {
    // Crée un nouvel allocateur avec un pool de mémoire donné
    fn new(pool_start: *mut u8, pool_size: usize) -> Self {
        MyAlloc {
            free_list: None,
            pool_start,
            pool_size,
        }
    }

    // initialise l'allocateur avec un bloc unique couvrant toute la mémoire
    unsafe fn init(&mut self) { // On le marque comme unsafe car la fonction write nécessite que ce soit spécifié
        let first_block: FreeBlock = FreeBlock {
            size: self.pool_size,
            next: None,
        };

        let first_block_ptr: *mut FreeBlock = self.pool_start as *mut FreeBlock;
        first_block_ptr.write(first_block);

        self.free_list = Some(NonNull::new(first_block_ptr).unwrap());
    }

    fn find_fit(&mut self, layout: Layout)/* -> *mut u8 */ {
        let mut prev = None;
        let mut current = self.free_list;

        while let Some(mut current_block) = current {
            let block = current_block.as_mut();

            if block.size > layout.size() + size_of::<FreeBlock>() {
                // On retire ce bloc de la free list
                if let Some(mut prev_block) = prev {
                    prev_block.as_mut().next = block.next;
                } else {
                    self.free_list = block.next;
                }

                // On sépare la mémoire si elle est plus grande que l'élément qu'on stocke
                let remaining_size = block.size - layout.size() - size_of::<FreeBlock>();

                if remaining_size >= size_of::<FreeBlock>() {
                    // On crée un nouveau bloc libre avec le reste de la mémoire
                    let new_free_block_ptr = (self.pool_start as usize + layout.size()) as *mut FreeBlock;
                    let new_free_block = FreeBlock {
                        size: remaining_size,
                        next: block.next,
                    }

                    new_free_block_ptr.write(new_free_block);
                    self.free_list = Some(NonNull::new(new_free_block_ptr).unwrap());
                }

                return current_block.as_ptr() as *mut u8;
            }

            prev = current;
            current = block.next;
        }

        // Pas assez de mémoire
        null_mut()
    }

    // Fonction pour libérer un bloc de mémoire
    fn dealloc(&mut self, ptr: *mut u8, layout: Layout) {
        let block_ptr = ptr as *mut FreeBlock;

        // Réinsérer un bloc dans la free_list
        let new_free_block = FreeBlock {
            size: layout.size(),
            next: self.free_list,
        };

        block_ptr.write(new_free_block);
        self.free_list = Some(NonNull::new(block_ptr).unwrap());

        // Fusionner les blocs adjaçants (pas implémenté encore)
    }
}

// Implémentation de GlobalAlloc dans notre allocateur custom
unsafe impl GlobalAlloc for MyAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        self.find_fit(layout)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        self.dealloc(ptr, layout)
    }
}

const POOL_SIZE: usize = 1024;
static mut POOL_START: [u8; POOL_SIZE] = [0, POOL_SIZE];

#[global_allocator]
static A: MyAlloc = MyAlloc::new(POOL_START, POOL_SIZE);

#[no_mangle]
pub extern "C" fn _start() {
    loop {}
}
