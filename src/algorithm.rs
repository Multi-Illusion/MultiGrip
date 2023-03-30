
pub fn cal_heart_rate(datas: &[u32], num: usize) -> i32 {
    let mut un_ir_mean: i32 = 0;
    let mut an_x = [0i32; 100];
    let mut an_ir_valley_locs = [0i32; 100];

    for i in 0..num { un_ir_mean += datas[i] as i32 }
    un_ir_mean = un_ir_mean / num as i32;

    for i in 0..num { an_x[i] = un_ir_mean - datas[i] as i32 }

    for i in 0..(num-4) { an_x[i] = (an_x[i] + an_x[i+1] + an_x[i+2] + an_x[i+3])/4 }
    
    let mut n_th1 = 0;
    for i in 0..(num-4) { n_th1 += an_x[i] }
    n_th1 = n_th1 / (num-4) as i32;

    if n_th1 < 30 {n_th1 = 30}
    if n_th1 > 60 {n_th1 = 60}


    let n_npks = find_peaks(&mut an_ir_valley_locs, &an_x, (num-4) as _, n_th1 as _, 4, 15);
    let mut n_peak_interval_sum = 0;
    if n_npks >= 2 {
        for k in 1..(n_npks as usize) { 
            n_peak_interval_sum += an_ir_valley_locs[k] - an_ir_valley_locs[k -1]
        }
        n_peak_interval_sum =n_peak_interval_sum/(n_npks-1) as i32;
        return (25*60)/ n_peak_interval_sum
    }

    return -999
    // return n_npks as _
}

fn find_peaks(
    pn_locs: &mut [i32;100],
    pn_x: &[i32;100],
    n_size: i32,
    n_min_height: i32,
    n_min_distance: i32,
    n_max_num: i32
) -> i32 {
    let n_npks = peaks_above_min_height(pn_locs, pn_x, n_size, n_min_height);
    let n_npks = remove_close_peaks(pn_locs, n_npks, pn_x, n_min_distance);
    if n_npks < n_max_num {
        return n_npks;
    }
    else {
        return n_max_num;
    }
}

fn peaks_above_min_height(
    pn_locs: &mut [i32;100],
    pn_x: &[i32;100],
    n_size: i32,
    n_min_height: i32,
) -> i32 {
    let mut i = 1;
    let mut n_width;
    let mut n_npks = 0;

    while i < ((n_size-1) as usize) {
        if (pn_x[i] > n_min_height) && (pn_x[i] > pn_x[i-1]) {
            n_width = 1;
            while (i+n_width < n_size as usize) && pn_x[i] == pn_x[i+n_width] { n_width += 1}
            if pn_x[i] > pn_x[i+n_width] && (n_npks) < 15 {     // find right edge of peaks
                pn_locs[n_npks] = i as i32;    
                n_npks += 1;
                // for flat peaks, peak location is left edge
                i += n_width+1;
            }
            else { i += n_width }
        }
        else { i += 1}
    }

    return n_npks as _
}

fn remove_close_peaks(
    pn_locs: &mut [i32;100],
    mut pn_npks: i32,
    pn_x: &[i32;100], n_min_distance: i32
) -> i32 {
    sort_indices_descend(pn_x, pn_locs, pn_npks);

    for i in (-1)..pn_npks {
        let n_old_npks = pn_npks;
        pn_npks = i + 1;
        for j in (i+1)..n_old_npks {
            let n_dist = pn_locs[j as usize] - ({
                if i == -1 { -1 } else { pn_locs[i as usize] }
            });

            if n_dist > n_min_distance || n_dist < -n_min_distance {
                pn_locs[pn_npks as usize] = pn_locs[j as usize];
                pn_npks += 1;
            }
        }
    }

    sort_ascend(pn_locs, pn_npks);

    pn_npks
}

fn sort_indices_descend(
    pn_x: &[i32;100],
    pn_indx: &mut [i32;100],
    n_size: i32
){
    for i in 1..(n_size as usize) {
        let n_temp = pn_indx[i];
        let mut j = i;
        while j > 0 && pn_x[n_temp as usize] > pn_x[pn_indx[j-1] as usize] {
            pn_indx[j] = pn_indx[j-1];
            j -= 1;
        }
        pn_indx[j] = n_temp;
    }
}

fn sort_ascend(pn_x: &mut [i32;100], n_size: i32) {
    for i in 1..(n_size as usize) {
        let n_temp = pn_x[i];
        let mut j = i;
        while j > 0 && n_temp < pn_x[j-1] {
            pn_x[j] = pn_x[j-1];
            j -= 1;
        }
        pn_x[j] = n_temp;
    }
}
