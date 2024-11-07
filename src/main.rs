extern crate ev3dev_lang_rust;
extern crate cmd_lib;
use ev3dev_lang_rust::sensors::{InfraredSensor, ColorSensor, SensorPort};
use ev3dev_lang_rust::motors::{LargeMotor, MediumMotor, MotorPort};
use ev3dev_lang_rust::{Ev3Result, Button};
use std::time::{SystemTime, Duration};
use cmd_lib::run_cmd;
use std::env;

// Three hard values used for moving away from the line
const DEFAULT_SPEED: i32 = 250;   // SUBJECT TO CHANGE
const CORRECTION_VALUE: i32 = 50; // SUBJECT TO CHANGE
const BLACK_THRESHOLD: i32 = 34;  // SUBJECT TO CHANGE
const GREEN_OFFSET: i32 = 25;     // SUBJECT TO CHANGE


fn main() -> Ev3Result<()> {
    // Motor Bindings
    let right_motor: LargeMotor = LargeMotor::get(MotorPort::OutA)?;
    let left_motor: LargeMotor = LargeMotor::get(MotorPort::OutB)?;
    let _winch: LargeMotor = LargeMotor::get(MotorPort::OutC)?;
    let _claw: MediumMotor = MediumMotor::get(MotorPort::OutD)?;
    
    // Sensor Bindings
    let left_sensor: ColorSensor = ColorSensor::get(SensorPort::In1)?;left_sensor.set_mode_rgb_raw()?;
    let right_sensor: ColorSensor = ColorSensor::get(SensorPort::In2)?;right_sensor.set_mode_rgb_raw()?;


    let infrared_sensor: InfraredSensor = InfraredSensor::get(SensorPort::In4)?;

    // Button


    // Initiating the motor speeds
    left_motor.set_speed_sp(DEFAULT_SPEED)?;
    right_motor.set_speed_sp(DEFAULT_SPEED)?;
    
    // Creates the Boolean Release Latch: Essentially a struct of two booleans, used as a latch 
    // that prevents button press events from triggering multiple times on the same press.
    let mut active:BoolReleaseLatch = BoolReleaseLatch::new(true);
    
    // A cooldown currently used for the green turn, to stop multiple triggers. Why didn't I use 
    // another Boolean Release Latch, you ask? :).
    let mut cooldown = SystemTime::now();

    // Collects all arguments in file running. currently used to avoid having to press the damn
    // button every time I want to start without actually removing that code because it's a 
    // useful function, damnit!
    let args: Vec<String> = env::args().collect();
    if !args.contains(&String::from("manual")) {
        active.set(true)
    }
    
    // A struct that gets repopulated every cycle with data to be printed to the terminal.
    let mut fields:DisplayTypes = DisplayTypes{ left_av:0, left_black_bool:false, left_green_bool:false, left_vals:(0,0,0), right_av:0, right_black_bool:false, right_green_bool:false, right_vals:(0,0,0), direction:String::from("")};

    // println!("Press the OK button to start the program"); //a reminder for the nerd who wrote this
    loop {
        // Engages the boolean and latch if a button press is detected (and the latch isn't enabled yet
        // if button.is_enter() && !active.latch {
        //     active.set(true);
        // }
        
        // Main Code
        if active.state {
            //Pre-algorithm Processing
            // button.process();
            
              //Left fields

            //Left Fields
            fields.left_black_bool = left_sensor.get_red().unwrap() < BLACK_THRESHOLD;
            fields.left_av = (left_sensor.get_green().unwrap() + left_sensor.get_red().unwrap())/2;
            fields.left_green_bool =  left_sensor.get_green().unwrap() - GREEN_OFFSET > left_sensor.get_red().unwrap();
            fields.left_vals = (left_sensor.get_red().unwrap(), left_sensor.get_green().unwrap(), left_sensor.get_blue().unwrap());


            //Right fields
            fields.right_black_bool = right_sensor.get_red().unwrap()/2 < BLACK_THRESHOLD;
            fields.right_av = right_sensor.get_red().unwrap()/2;
            fields.right_green_bool =  right_sensor.get_green().unwrap() - GREEN_OFFSET > right_sensor.get_red().unwrap();
            fields.right_vals = (right_sensor.get_red().unwrap(), right_sensor.get_green().unwrap(), right_sensor.get_blue().unwrap());

            // Checks if you've pressed the button for a second time; if so, terminates
            // the program safely.
            // if button.is_enter() && !active.latch {
            //     left_motor.stop().unwrap();
            //     right_motor.stop().unwrap();
            //     std::process::exit(0);
            // }
            //
            // // Releases the latch
            // if !button.is_enter() {
            //     active.release();
            // }
            
            // If it's been more than 3 seconds since the last green turn, checks for a green turn
            if cooldown.elapsed().unwrap_or_else(|_| Duration::new(0, 0)) > Duration::new(3, 0) {
                match can_see_green(&left_sensor, &right_sensor) {
                    1 => { // Does a 180° turn 
                        left_motor.set_speed_sp(DEFAULT_SPEED)?;
                        right_motor.set_speed_sp(-DEFAULT_SPEED)?;

                        left_motor.run_to_rel_pos(Some(500))?;
                        right_motor.run_to_rel_pos(Some(500))?;
                    },
                    2 => { // Does a 90° left turn
                        left_motor.set_speed_sp(0)?;
                        right_motor.set_speed_sp(DEFAULT_SPEED)?;

                        left_motor.run_to_rel_pos(None)?;
                        right_motor.run_to_rel_pos(Some(500))?;
                    },
                    3 => { // Does a 90° right turn
                        left_motor.set_speed_sp(DEFAULT_SPEED)?;
                        right_motor.set_speed_sp(0)?;

                        left_motor.run_to_rel_pos(Some(500))?;
                        right_motor.run_to_rel_pos(None)?;
                    },
                    _ => continue
                }
                cooldown = SystemTime::now(); // updates the cooldown to the present
            }
            
            // Determines the direction of the robot
            match can_see_black(&left_sensor, &right_sensor) {
                0 => { // Forward
                    fields.direction = String::from("forward");
                    right_motor.set_speed_sp(DEFAULT_SPEED)?;
                    left_motor.set_speed_sp(DEFAULT_SPEED)?;
                    left_motor.run_forever()?;
                    right_motor.run_forever()?;
                    
                }
                1 => { // Right
                    fields.direction = String::from("right");
                    left_motor.set_speed_sp(CORRECTION_VALUE * 6)?;
                    right_motor.set_speed_sp(DEFAULT_SPEED)?;
                    left_motor.run_to_rel_pos(Some(-CORRECTION_VALUE))?;
                }
                2 => { // Left
                    fields.direction = String::from("left");
                    right_motor.set_speed_sp(CORRECTION_VALUE * 3)?;
                    left_motor.set_speed_sp(DEFAULT_SPEED)?;
                    right_motor.run_to_rel_pos(Some(-CORRECTION_VALUE))?;
                }

                _ => continue
            }
            
            // Prints the latest instance of the data to the screen
            fields.display();
        }
        // 
        if infrared_sensor.get_distance().unwrap_or_else(|_| 0) < 90 {
            water_tower()
        }
        
            
        // Update the button, because you gotta do that manually apparently 😒
        else {
            // button.process();
        }
    }
}


fn can_see_black(left_sensor:&ColorSensor, right_sensor:&ColorSensor) -> i32 {
    // Averages both sensors, then returns a value based on how they compare to a constant black value
    if (left_sensor.get_blue().unwrap_or_else(|_| 0) + left_sensor.get_green().unwrap_or_else(|_| 0) + left_sensor.get_red().unwrap_or_else(|_| 0))/3 < BLACK_THRESHOLD &&
        (right_sensor.get_blue().unwrap_or_else(|_| 0) + right_sensor.get_green().unwrap_or_else(|_| 0) + right_sensor.get_red().unwrap_or_else(|_| 0))/3 < BLACK_THRESHOLD
    {
        
        return 0 // Double Black
    }
    if (left_sensor.get_blue().unwrap_or_else(|_| 0) + left_sensor.get_green().unwrap_or_else(|_| 0) + left_sensor.get_red().unwrap_or_else(|_| 0))/3 < BLACK_THRESHOLD
    {
        return 1 // Left Black
    }
    if (right_sensor.get_blue().unwrap_or_else(|_| 0) + right_sensor.get_green().unwrap_or_else(|_| 0) + right_sensor.get_red().unwrap_or_else(|_| 0))/3 < BLACK_THRESHOLD
    {
        return 2 //Right Black
    }
    return 0

}
fn can_see_green(left_sensor:&ColorSensor, right_sensor:&ColorSensor) -> i32 {
    // Gets the difference between the green value and the average of the red & blue value, then
    // returns a value based on how they compare to a constant green threshold value
    if left_sensor.get_green().unwrap_or_else(|_| 0) + GREEN_OFFSET > left_sensor.get_blue().unwrap_or_else(|_| 0) + left_sensor.get_red().unwrap_or_else(|_| 0)/2 &&
        right_sensor.get_green().unwrap_or_else(|_| 0) + GREEN_OFFSET > right_sensor.get_blue().unwrap_or_else(|_| 0) + right_sensor.get_red().unwrap_or_else(|_| 0)/2
    {
        return 1 // Do a 180° turn
    }

    if left_sensor.get_green().unwrap_or_else(|_| 0) + GREEN_OFFSET > left_sensor.get_blue().unwrap_or_else(|_| 0) + left_sensor.get_red().unwrap_or_else(|_| 0)/2 {
        return 2 // Do a 90° left turn
    }
    if right_sensor.get_green().unwrap_or_else(|_| 0) + GREEN_OFFSET > right_sensor.get_blue().unwrap_or_else(|_| 0) + right_sensor.get_red().unwrap_or_else(|_| 0)/2 {
        return 3 // Do a 90° right turn
    }
    0 // Nothing :3
}

fn water_tower() {
    // Handles moving around the water tower. Match statement contains different methods
    // of avoiding the tower, determined by the constant immediately above it.
    const WATER_TOWER_MODE:i32 = 0;
    match WATER_TOWER_MODE {
        0 => {}, // Follows a hard-coded path around the water tower
        1 => {}, // Uses the claw to shift the water tower out of the way
        _ => {}  // Default value
    }
}

fn _chemical_spill(_left_sensor: &ColorSensor, _right_sensor: &ColorSensor, _infrared_sensor: &InfraredSensor) {
    // Forward for half a tile
    
    // Spin in a circle
    // Take readings every cycle until the circle is complete
    // within the vec, find the start of a segment of true readings - continue until they become 
    // false. then, go halfway between those points & set everything other than that one to false.
    // Do so for every 
    
}

// A struct containing two booleans - one the actual value, the other a latch that stops
// considerations based on the first bool being made too many times.
struct BoolReleaseLatch {
    state: bool,
    latch: bool
}

// Functions designed to do exactly what the name suggests, involving
impl BoolReleaseLatch {
    fn new(state:bool) -> Self{Self{state:state, latch:false}}
    fn set(&mut self, state:bool) {self.state = state; self.latch = state}
    fn release(&mut self) {self.latch = false}
}

// A series of values to be printed every cycle displaying valuable diagnostic variables
pub struct DisplayTypes {
    left_av: i32,
    left_black_bool: bool,
    left_green_bool: bool,
    left_vals: (i32, i32, i32),


    right_av: i32,
    right_black_bool: bool,
    right_green_bool: bool,
    right_vals: (i32, i32, i32),


    direction: String

}

// prints the aforementioned variables
impl DisplayTypes {
    fn display(&self) -> () {
        run_cmd!(clear).expect("Screen failed to clear");
            print!("Direction: \x1b[36m{:?}\x1b[0m\n", self.direction);
        
            print!("Left value: \x1b[36m{}\x1b[0m\t", self.left_av,);
            print!("Black: {}{}\x1b[0m\t", if self.left_black_bool {"\x1b[32m"} else {"\x1b[31m\t"}, self.left_black_bool);
            print!("Green: {}{}\x1b[0m", if self.left_green_bool {"\x1b[32m"} else {"\x1b[31m"}, self.left_green_bool);
            print!("\t\x1b[36m{:?}\x1b[0m\n", self.left_vals);

            print!("Right value: \x1b[36m{}\x1b[0m\t", self.right_av,);
            print!("Black: {}{}\x1b[0m\t", if self.right_black_bool {"\x1b[32m"} else {"\x1b[31m\t"}, self.right_black_bool);
            print!("Green: {}{}\x1b[0m", if self.right_green_bool {"\x1b[32m"} else {"\x1b[31m"}, self.right_green_bool);
            print!("\t\x1b[36m{:?}\x1b[0m\n", self.right_vals);
    }
}