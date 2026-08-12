// 1. تعريف متغيرات أساسية للاختبار
let int(32) number = 42;
//let length my_array = [1, 2, 3];

// 2. اختبار الـ Pointers (الـ name)
// مؤشر للقراءة فقط (ReadOnly)
let name safe_ptr =  number; 

// مؤشر قابل للتعديل (ReadWrite)
let name mut_ptr = modify number; 

// 3. اختبار خصائص الذاكرة (length, size, data)
//let length arr_len =  my_array;
//let size arr_size =  my_array;

// استخراج الـ Raw Data Pointer
//let data arr_data =  my_array; 

// 4. اختبار معرفة النوع وقت الترجمة
//let type num_type =  number;