fn helper() {
}

fn compute(mut base:i32, limit:i32) -> i32 {
    ;
    let plain;
    let typed:i32;
    let mut value:i32=base+1*2;
    let arr:[i32;3]=[1,2,3];
    let mut pair:(i32,i32)=(arr[0], value);
    let unit:()=();
    let ref_value:&i32=&value;
    let mut ref_mut:&mut i32=&mut value;

    plain=0;
    typed=(plain+value)/2;
    pair.0=typed;
    *ref_mut=pair.0-1;
    arr[1];
    pair.1;
    helper();
    helper(value, arr[2]);
    -value;
    &value;
    &mut value;
    *ref_mut;
    { let t:i32=1; t };

    if value>=limit {
        value=value-1;
    } else if value!=0 {
        value=value+1;
    } else {
        return;
    }

    while value>0 {
        if value==2 {
            break;
        }
        value=value-1;
    }

    for mut i in 0..limit {
        if i<=1 {
            continue;
        }
    }

    for item in arr {
        item;
    }

    loop {
        break;
    }

    let loop_value=loop {
        break 7;
    };
    let chosen=if value<limit { value } else { limit };

    return chosen;
}
#
