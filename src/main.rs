#![no_std]
#![no_main]

use core::panic::PanicInfo;
use core::ptr::null_mut;
use core::alloc::{GlobalAlloc, Layout};
use core::ptr::NonNull;
use core::mem::size_of;
use core::cell::UnsafeCell;

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

#[no_mangle]
pub extern "C" fn _start() -> ! {
    loop {}
}
