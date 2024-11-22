use colored::*;

pub fn print_startup_art() {
    let bee_art = r#"
                          __
                         // \
                         \\_/ //
    brrr''-.._.-''-.._.. -(||)(')
                         '''  
    "#;

    println!("{}", bee_art.bold().bright_yellow());
}

pub fn print_exit_art() {
    let bear_art = r#"
        _
     __( )_
    (      (o____
     |          |
     |      (__/
       \     /   ___
       /     \  \___/
     /    ^    /     \
    |   |  |__|_ SHA  |
    |    \______)____/
     \         /
       \     /_
        |  ( __)
        (____)
        "#;

    println!("{}", bear_art.bold().bright_yellow());
}

pub fn display_share_accepted() {
    let ascii_art = r#"
      .             *        .     .       .
           .     _     .     .            .       .
    .    .   _  / |      .        .  *         _  .     .
            | \_| |                           | | __
          _ |     |                   _       | |/  |
         | \      |      ____        | |     /  |    \
         |  |     \    +/_\/_\+      | |    /   |     \
    ____/____\--...\___ \_||_/ ___...|__\-..|____\____/__
          .     .      |_|__|_|         .       .
       .    . .       _/ /__\ \_ .          .
          .       .    .           .         . 
                                             ___
                                          .-' \\".
                                         /`    ;--:
                                        |     (  (_)==
                                        |_ ._ '.__.;
                                        \_/`--_---_(
                                         (`--(./-\.)
                                         `|     _\ |
                                          | \  __ /
                                         /|  '.__/
                                      .'` \     |_
                                           '-__ / `-           
    "#;

    println!("{}", ascii_art.bold().green());
}
