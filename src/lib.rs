use std::cmp::{max, min};
use std::error::Error;

#[derive(Debug)]
pub struct Limit {
    pub iops: u32,
    pub speed: u32,
    pub burst_iops: u32,
    pub burst_speed: u32,
}

impl Limit {
    pub fn default() -> Limit {
        Limit{
            iops: 0,
            speed: 0,
            burst_iops: 0,
            burst_speed: 0
        }
    }
}

pub fn calculate_gp2_limits(volume_size_gb: u32) -> Result<Limit, Box<dyn Error>> {
    if (volume_size_gb < 1) || (volume_size_gb > 16384) {
        return Err("Volume size for gp2 can not be less than 1GiB or greater than 16384GiB")?;
    }

    if volume_size_gb > 1000 {
        let max_available_iops = 16000;                // Max IOPS available for this volume type
        let max_available_throughput = 250;            // Max throughput available for this volume type
        let calculate_iops = 3 * volume_size_gb;
        let baseline_iops = min(calculate_iops, max_available_iops);             // Baseline for Gp2 can not be more than max_available_iops
        let baseline_throughput = max_available_throughput;   // For volumes greater than 1000GiB, max throughput is always 250MiB/s.
        return Ok(Limit { iops: baseline_iops, speed: baseline_throughput, burst_iops: 0, burst_speed: 0 });
    } else {
        let burst = 3000;
        if volume_size_gb < 170 {
            let max_available_throughput = 128;                            // Gp2 volumes of size less than 170GiB have a throughput cap at 128MiB/s
            let calculate_iops = 3 * volume_size_gb;
            let baseline_iops = max(calculate_iops, 100);                // Baseline for Gp2 can not be less than 100.
            let calculate_tp = baseline_iops / 4;  // Calculating throughput from IOPS with max block size as 256KiB
            let baseline_throughput = min(max_available_throughput, calculate_tp);      // Throughput can not exceed max_available_throughput
            return Ok(Limit { iops: baseline_iops, speed: baseline_throughput, burst_iops: burst, burst_speed: max_available_throughput });
        } else {
            let max_available_throughput = 250;
            let calculate_iops = 3 * volume_size_gb;
            let baseline_iops = calculate_iops;
            let calculate_tp = baseline_iops / 4;
            let baseline_throughput = min(max_available_throughput, calculate_tp);      // Throughput can not exceed max_available_throughput
            return Ok(Limit { iops: baseline_iops, speed: baseline_throughput, burst_iops: burst, burst_speed: max_available_throughput });
        }
    }
}


pub fn calculate_gp3_limits(volume_size_gb: u32, volume_provisioned_iops: Option<u32>, volume_provisioned_throughput: Option<u32>) -> Result<Limit, Box<dyn Error>> {
    if (volume_size_gb < 1) || (volume_size_gb > 16384) {
        return Err("Volume size for gp3 can not be less than 1GiB or greater than 16384GiB")?;
    }

    let volume_iops = if volume_provisioned_iops.is_none() {
        // Set IOPS as Baseline(3000) for gp3 volume if it was created using CLI without provisioned IOPS .
        3000
    } else {
        let iops = volume_provisioned_iops.unwrap();
        if (iops < 3000) || (iops > 64000) {
            return Err("Provisioned IOPS can not be less than 3000 or greater than 16000 for Gp3 volume type..")?;
        }

        if iops / volume_size_gb > 500 {
            return Err("Maximum ratio of 500:1 is permitted between IOPS and volume size for Gp3 volume type.")?;
        }

        iops
    };

    let volume_throughput = if volume_provisioned_throughput.is_none() {
        // Set Throughput as Baseline(125MiB/s) for gp3 volume if it was created using CLI without provisioned Throughput .
        125
    } else {
        let throughput = volume_provisioned_throughput.unwrap();
        if (throughput < 125) || (throughput > 1000) {
            return Err("Provisioned throughput can not be less than 125MiB/s or greater than 1000MiB/s for Gp3 volume type..")?;
        }
        if volume_iops / throughput < 4 {
            return Err("Maximum ratio of 0.25:1 is permitted between Throughput (MiBps) and IOPS for Gp3 volume type.")?;
        }
        throughput
    };
    return Ok(Limit { iops: volume_iops, speed: volume_throughput, burst_iops: 0, burst_speed: 0 });
}

pub fn calculate_io_limits(volume_provisioned_iops: u32) -> Result<Limit, Box<dyn Error>> {
    if (volume_provisioned_iops < 100) || (volume_provisioned_iops > 64000) {
        return Err("Provisioned IOPS can not be less than 100 or greater than 64000.")?;
    }

    let baseline_throughput;
    if volume_provisioned_iops < 32000 {
        let max_available_throughput = 500;                            // io1/io2 Volumes with less than equal to 32000 provisioned IOPS can achieve 500MiB/s of throughput at max.
        let calculate_tp = volume_provisioned_iops / 4;
        baseline_throughput = min(max_available_throughput, calculate_tp);
    } else {
        let max_available_throughput = 1000;
        let calculate_tp = volume_provisioned_iops / 64;    // io1/io2 volume provisioned with more than 32,000 IOPS supports a maximum I/O size of 16 KiB
        baseline_throughput = min(max_available_throughput, calculate_tp);
    }
    return Ok(Limit { iops: volume_provisioned_iops, speed: baseline_throughput, burst_iops: 0, burst_speed: 0 });
}

// todo: calculate_st1_limits;
// https://github.com/awslabs/aws-support-tools/blob/master/EBS/VolumeLimitCalculator/volume_Limit_calculator.sh#L194

// todo: calculate_sc1_limits
// https://github.com/awslabs/aws-support-tools/blob/master/EBS/VolumeLimitCalculator/volume_Limit_calculator.sh#L236

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gp2_1500() {
        let limit = calculate_gp2_limits(1500).unwrap();
        assert_eq!(limit.iops, 4500);
        assert_eq!(limit.speed, 250);
        assert_eq!(limit.burst_iops, 0);
        assert_eq!(limit.burst_speed, 0);
    }

    #[test]
    fn test_gp2_20() {
        let limit = calculate_gp2_limits(20).unwrap();
        assert_eq!(limit.iops, 100);
        assert_eq!(limit.speed, 25);
        assert_eq!(limit.burst_iops, 3000);
        assert_eq!(limit.burst_speed, 128);
    }

    #[test]
    fn test_gp2_1000() {
        let limit = calculate_gp2_limits(1000).unwrap();
        assert_eq!(limit.iops, 3000);
        assert_eq!(limit.speed, 250);
        assert_eq!(limit.burst_iops, 3000);
        assert_eq!(limit.burst_speed, 250);
    }

    #[test]
    fn test_gp2_10000() {
        let limit = calculate_gp2_limits(10000).unwrap();
        assert_eq!(limit.iops, 16000);
        assert_eq!(limit.speed, 250);
        assert_eq!(limit.burst_iops, 0);
        assert_eq!(limit.burst_speed, 0);
    }

    #[test]
    fn test_gp2_3000() {
        let limit = calculate_gp2_limits(3000).unwrap();
        assert_eq!(limit.iops, 9000);
        assert_eq!(limit.speed, 250);
        assert_eq!(limit.burst_iops, 0);
        assert_eq!(limit.burst_speed, 0);
    }

    #[test]
    fn test_gp3_1500() {
        let limit = calculate_gp3_limits(1500, None, None).unwrap();
        assert_eq!(limit.iops, 3000);
        assert_eq!(limit.speed, 125);
        assert_eq!(limit.burst_iops, 0);
        assert_eq!(limit.burst_speed, 0);
    }

    #[test]
    fn test_gp3_20() {
        let limit = calculate_gp3_limits(20, None, None).unwrap();
        assert_eq!(limit.iops, 3000);
        assert_eq!(limit.speed, 125);
        assert_eq!(limit.burst_iops, 0);
        assert_eq!(limit.burst_speed, 0);
    }

    #[test]
    fn test_gp3_1000() {
        let limit = calculate_gp3_limits(1000, None, None).unwrap();
        assert_eq!(limit.iops, 3000);
        assert_eq!(limit.speed, 125);
        assert_eq!(limit.burst_iops, 0);
        assert_eq!(limit.burst_speed, 0);
    }

    #[test]
    fn test_gp3_10000() {
        let limit = calculate_gp3_limits(10000, None, None).unwrap();
        assert_eq!(limit.iops, 3000);
        assert_eq!(limit.speed, 125);
        assert_eq!(limit.burst_iops, 0);
        assert_eq!(limit.burst_speed, 0);
    }

    #[test]
    fn test_io1_1500() {
        let limit = calculate_io_limits(1500).unwrap();
        assert_eq!(limit.iops, 1500);
        assert_eq!(limit.speed, 375);
        assert_eq!(limit.burst_iops, 0);
        assert_eq!(limit.burst_speed, 0);
    }

    #[test]
    fn test_io1_20() {
        assert_eq!(calculate_io_limits(20).is_err(), true);
    }

    #[test]
    fn test_io1_1000() {
        let limit = calculate_io_limits(1000).unwrap();
        assert_eq!(limit.iops, 1000);
        assert_eq!(limit.speed, 250);
        assert_eq!(limit.burst_iops, 0);
        assert_eq!(limit.burst_speed, 0);
    }

    #[test]
    fn test_io1_10000() {
        let limit = calculate_io_limits(10000).unwrap();
        assert_eq!(limit.iops, 10000);
        assert_eq!(limit.speed, 500);
        assert_eq!(limit.burst_iops, 0);
        assert_eq!(limit.burst_speed, 0);
    }
}
