#![no_std]
#![no_main]

extern crate libc;

use core::panic::PanicInfo;
use core::ptr::null_mut;
use core::alloc::{GlobalAlloc, Layout};
use core::ptr::NonNull;
use core::mem::size_of;
use core::cell::UnsafeCell;
use core::fmt::{self, Write};

// Taille du pool de mémoire pour l'allocateur (exemple de 1024 octets)
const POOL_SIZE: usize = 1024;

// Déclare un buffer statique pour servir de pool de mémoire
static mut MEMORY_POOL: [u8; POOL_SIZE] = [0; POOL_SIZE];

#[panic_handler]
fn panic(_panic: &PanicInfo) -> ! {
    loop {}
}

// Structure pour représenter un bloc libre dans la free list
struct FreeBlock {
    size: usize,                       // Taille du bloc libre
    next: Option<NonNull<FreeBlock>>,   // Pointeur vers le bloc libre suivant
}

// L'allocateur en lui-même, basé sur une free list
pub struct FreeListAllocator {
    free_list: UnsafeCell<Option<NonNull<FreeBlock>>>, // Liste chaînée des blocs libres
    pool_start: *mut u8,                               // Début du pool de mémoire
    pool_size: usize,                                  // Taille du pool de mémoire
}

impl FreeListAllocator {
    /// Crée un nouvel allocateur avec un pool de mémoire donné
    pub const fn new(pool_start: *mut u8, pool_size: usize) -> Self {
        FreeListAllocator {
            free_list: UnsafeCell::new(None),
            pool_start,
            pool_size,
        }
    }

    /// Initialise l'allocateur avec un bloc unique couvrant toute la mémoire
    pub unsafe fn init(&self) {
        let first_block = FreeBlock {
            size: self.pool_size,
            next: None,
        };

        let first_block_ptr = self.pool_start as *mut FreeBlock;
        first_block_ptr.write(first_block);

        *self.free_list.get() = Some(NonNull::new(first_block_ptr).unwrap());
    }

    /// Fonction pour trouver un bloc libre et l'allouer
    unsafe fn find_fit(&self, layout: Layout) -> *mut u8 {
        let mut prev: Option<NonNull<FreeBlock>> = None;
        let mut current = *self.free_list.get();

        while let Some(mut current_block) = current {
            let block = current_block.as_mut();
            
            // Vérifier la taille et l'alignement
            if block.size >= layout.size() + size_of::<FreeBlock>() && (current_block.as_ptr() as usize) % layout.align() == 0 {
                // Retirer ce bloc de la free list
                if let Some(mut prev_block) = prev {
                    prev_block.as_mut().next = block.next;
                } else {
                    *self.free_list.get() = block.next;
                }

                // Séparer la mémoire si nécessaire
                let remaining_size = block.size - layout.size() - size_of::<FreeBlock>();

                if remaining_size >= size_of::<FreeBlock>() {
                    // Créer un nouveau bloc libre avec le reste
                    let new_free_block_ptr = (current_block.as_ptr() as usize + layout.size()) as *mut FreeBlock;
                    let new_free_block = FreeBlock {
                        size: remaining_size,
                        next: block.next,
                    };

                    new_free_block_ptr.write(new_free_block);
                    *self.free_list.get() = Some(NonNull::new(new_free_block_ptr).unwrap());
                }

                return current_block.as_ptr() as *mut u8;
            }

            prev = current;
            current = block.next;
        }

        // Pas assez de mémoire
        null_mut()
    }

    /// Fonction pour libérer un bloc de mémoire
    pub unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let block_ptr = ptr as *mut FreeBlock;

        // Réinsérer le bloc dans la free list
        let new_free_block = FreeBlock {
            size: layout.size(),
            next: *self.free_list.get(),
        };

        block_ptr.write(new_free_block);
        *self.free_list.get() = Some(NonNull::new(block_ptr).unwrap());

        // Fusionner avec les blocs adjacents si possible (pas implémenté ici)
    }
}

// Implémentation de GlobalAlloc pour le free list allocator
unsafe impl GlobalAlloc for FreeListAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        self.find_fit(layout)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        self.dealloc(ptr, layout)
    }
}

// Implémentation du trait Sync pour permettre à notre allocateur static d'être partagé entre les threads
unsafe impl Sync for FreeListAllocator {}

struct Writer;

impl Write for Writer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for byte in s.as_bytes() {
            unsafe {
                put_char(*byte);
            }
        }
        Ok(())
    }
}

unsafe fn put_char(c: u8) {
    libc::write(1, &c as *const u8 as *const _, 1);
}

// Fonction appelée en cas d'erreurs, on l'initialise car en no_std, on doit créer notre propre fonction.
#[no_mangle]
pub extern "C" fn rust_eh_personality() {}

// Création d'une instance de l'allocateur
#[global_allocator]
static GLOBAL_ALLOCATOR: FreeListAllocator = FreeListAllocator::new(unsafe { MEMORY_POOL.as_ptr() as *mut u8 }, POOL_SIZE);

#[no_mangle]
pub extern "C" fn _start() -> ! {

    // Initialisation de l'allocateur
    unsafe {
        GLOBAL_ALLOCATOR.init();
    }

    let a = 10;
    let b = 20;
    let c = a + b;
    
    let mut writer = Writer;

    write!(writer, "c = {}\n", c).unwrap();

    loop {}
}

#[cfg(miri)]
#[no_mangle]
fn miri_start(_argc: isize, _argv: *const *const u8) -> isize {
    // Appeler la fonction _start lors de l'exécution avec Miri
    _start();
}
