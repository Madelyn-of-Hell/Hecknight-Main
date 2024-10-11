extern crate ev3dev_lang_rust;
use std::env;
use std::time::{SystemTime, Duration};
use ev3dev_lang_rust::{Ev3Result, Button};
use ev3dev_lang_rust::motors::{LargeMotor, MediumMotor, MotorPort};
use ev3dev_lang_rust::sensors::{InfraredSensor, ColorSensor, SensorPort};



fn main() -> Ev3Result<()> {
    // Motor Bindings
    let right_motor: LargeMotor = LargeMotor::get(MotorPort::OutA)?;
    let left_motor: LargeMotor = LargeMotor::get(MotorPort::OutB)?;
    let _winch: LargeMotor = LargeMotor::get(MotorPort::OutC)?;
    let _claw: MediumMotor = MediumMotor::get(MotorPort::OutD)?;
    
    // Sensor Bindings
    let left_sensor: ColorSensor = ColorSensor::get(SensorPort::In1)?;
    let right_sensor: ColorSensor = ColorSensor::get(SensorPort::In2)?;

    let infrared_sensor: InfraredSensor = InfraredSensor::get(SensorPort::In4)?;
    // Button
    let button = Button::new()?;
    
    // Two hard values used for moving away from the line
    const DEFAULT_SPEED: i32 = 250; // SUBJECT TO CHANGE
    const CORRECTION_VALUE: i32 = 50;

    // Initiating the motor speeds
    left_motor.set_speed_sp(DEFAULT_SPEED)?;
    right_motor.set_speed_sp(DEFAULT_SPEED)?;
    
    // Creates the Boolean Release Latch: Essentially a struct of two booleans, used as a latch 
    // that prevents button press events from triggering multiple times on the same press.
    let mut active:BoolReleaseLatch = BoolReleaseLatch::new(false);
    
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
    let mut fields:DisplayTypes = DisplayTypes{ left_val:0,left_bool:false,right_val:0,right_bool:false,direction:String::from("")};

    println!("Press the OK button to start the program"); //a reminder for the nerd who wrote this
    loop {
        // Engages the boolean and latch if a button press is detected (and the latch isn't enabled yet
        if button.is_enter() && !active.latch {
            active.set(true);
        }
        
        // Main Code
        if active.state {
            //Pre-algorithm Processing
            button.process();
            
              //Left fields
              fields.left_bool = (left_sensor.get_blue().unwrap_or_else(|_| 0) + left_sensor.get_green().unwrap_or_else(|_| 0) + left_sensor.get_red().unwrap_or_else(|_| 0))/3 < 10; 
              fields.left_val = (left_sensor.get_blue().unwrap_or_else(|_| 0) + left_sensor.get_green().unwrap_or_else(|_| 0) + left_sensor.get_red().unwrap_or_else(|_| 0))/3;
              
              //Right fields
              fields.right_bool = (right_sensor.get_blue().unwrap_or_else(|_| 0) + right_sensor.get_green().unwrap_or_else(|_| 0) + right_sensor.get_red().unwrap_or_else(|_| 0))/3 < 10; 
              fields.right_val = (right_sensor.get_blue().unwrap_or_else(|_| 0) + right_sensor.get_green().unwrap_or_else(|_| 0) + right_sensor.get_red().unwrap_or_else(|_| 0))/3;
            
            // Checks if you've pressed the button for a second time; if so, terminates 
            // the program safely.
            if button.is_enter() && !active.latch {
                left_motor.stop().unwrap();
                right_motor.stop().unwrap();
                std::process::exit(0);
            }
            
            // Releases the latch
            if !button.is_enter() {
                active.release();
            }
            
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
                    left_motor.set_speed_sp(CORRECTION_VALUE * 3)?;
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
            display(&fields);
        }
        if infrared_sensor.get_distance().unwrap_or_else(|_| 0) < 90 {
            water_tower()
        }
        
            
        // Update the button, because you gotta do that manually apparently
        else {
            button.process();
        }
    }
}


fn can_see_black(left_sensor:&ColorSensor, right_sensor:&ColorSensor) -> i32 {
    if (left_sensor.get_blue().unwrap_or_else(|_| 0) + left_sensor.get_green().unwrap_or_else(|_| 0) + left_sensor.get_red().unwrap_or_else(|_| 0))/3 < 10 &&
        (right_sensor.get_blue().unwrap_or_else(|_| 0) + right_sensor.get_green().unwrap_or_else(|_| 0) + right_sensor.get_red().unwrap_or_else(|_| 0))/3 < 10
    {
        
        return 0 // Double Black
    }
    if (left_sensor.get_blue().unwrap_or_else(|_| 0) + left_sensor.get_green().unwrap_or_else(|_| 0) + left_sensor.get_red().unwrap_or_else(|_| 0))/3 < 10
    {
        return 1 // Left Black
    }
    if (right_sensor.get_blue().unwrap_or_else(|_| 0) + right_sensor.get_green().unwrap_or_else(|_| 0) + right_sensor.get_red().unwrap_or_else(|_| 0))/3 < 10
    {
        return 2 //Right Black
    }
    return 0

}
fn can_see_green(left_sensor:&ColorSensor, right_sensor:&ColorSensor) -> i32 {
    if left_sensor.get_green().unwrap_or_else(|_| 0) + 50 > left_sensor.get_blue().unwrap_or_else(|_| 0) + left_sensor.get_red().unwrap_or_else(|_| 0)/2 &&
        right_sensor.get_green().unwrap_or_else(|_| 0) + 50 > right_sensor.get_blue().unwrap_or_else(|_| 0) + right_sensor.get_red().unwrap_or_else(|_| 0)/2
    {
        return 1 // Do a 180° turn
    }

    if left_sensor.get_green().unwrap_or_else(|_| 0) + 50 > left_sensor.get_blue().unwrap_or_else(|_| 0) + left_sensor.get_red().unwrap_or_else(|_| 0)/2 {
        return 2 // Do a 90° left turn
    }
    if right_sensor.get_green().unwrap_or_else(|_| 0) + 50 > right_sensor.get_blue().unwrap_or_else(|_| 0) + right_sensor.get_red().unwrap_or_else(|_| 0)/2 {
        return 3 // Do a 90° right turn
    }
    0 // Nothing :3
}

fn water_tower() {
    const WATER_TOWER_MODE:i32 = 0;
    match WATER_TOWER_MODE {
        0 => {},
        _ => {}
    }
}

struct BoolReleaseLatch {
    state: bool,
    latch: bool
}
impl BoolReleaseLatch {
    fn new(state:bool) -> Self{Self{state:state, latch:false}}
    fn set(&mut self, state:bool) {self.state = state; self.latch = state}
    fn release(&mut self) {self.latch = false}
}

pub struct DisplayTypes {
    left_val: i32,
    left_bool: bool,

    right_val: i32,
    right_bool: bool,

    direction: String

}

fn display(fields: &DisplayTypes) -> () {
    println!("\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n");
        println!("Direction: \x1b[36m{:?}\x1b[0m", fields.direction);
        println!("Left value: \x1b[36m{}\x1b[0m\tBlack: {}{}\x1b[0m", fields.left_val,if fields.left_bool {"\x1b[32m"} else {"\x1b[31m"}, fields.left_bool);
        println!("Right value: \x1b[36m{}\x1b[0m\tBlack: {}{}\x1b[0m", fields.right_val,if fields.right_bool {"\x1b[32m"} else {"\x1b[31m"}, fields.right_bool);
}