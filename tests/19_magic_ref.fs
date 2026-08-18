fn main() -> void {
    int(32) number = 42;
    int(32) my_array[5] = [1,2,3,4,5];

    name safe_ptr = number; 

    name mut_ptr -> modify number; 

    int(32)* my_heap_array[5] = new int(32)[1,2,3,4,5];
    
    name not_safe_ptr -> new int(32)[1,2,3,4,5];
     del not_safe_ptr;  // that will delete the temp value and make the name empty
    not_safe_ptr -> my_array;   //it doesn't work *error: assignment of read-only reference 'not_safe_ptr'*
    
    name<int(32)> bad_ptr -> safe_ptr;   //Warning : nested and linked ptr you must to be manged correctly
    log(*bad_ptr);
    log(*mut_ptr);
}