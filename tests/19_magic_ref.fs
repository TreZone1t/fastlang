fn main() -> void {
    int(32) number = 42;
    int(32) my_array[5] = [1,2,3,4,5];

    name safe_ptr = number; 

    name mut_ptr -> modify number; 
    
    name not_safe_ptr -> [1,2,3,4,5];
    // del not_safe_ptr;  // that will delete the temp value and make the name empty
    not_safe_ptr -> my_array;   // Error: must delete a temp value before reassigning to insure memory safety
    
    name bad_ptr -> safe_ptr;   //Warning : nested and linked ptr you must to be manged correctly
}