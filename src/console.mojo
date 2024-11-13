from stash import USeg, FArr

fn main():  
    var     uSeg = USeg( 0, 1)  
    var     vSeg = uSeg;
    @parameter
    fn  trial( k: UInt32)  -> None:     
        print( repr( vSeg))

    uSeg.Traverse[ trial]()

    

