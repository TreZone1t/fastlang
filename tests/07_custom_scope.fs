// Valid: complete custom scope
scope Box -> {
    enable [length , keyword];
    type -> custom("box");
    param -> {
        int(32) width;
        int(32) height;
    }
    _()-> {
        this.width -> 10;
        this.height -> 20;
    }
    public ->{
        fn get_volume() -> int(32) {
            return width * height * height;
        }
    }
    //let box(10,20) myBox;
   // box(10,20) myBox; 
}
