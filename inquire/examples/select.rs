use inquire::Select;

fn main() {
    let options = vec![
        " [🏃‍♂️ Needs a reason] |  Some text to use up space  ⬅  More text to use up space ⬅  More and more and more text to use up space ⬅  More and more text to use up space ⬅  More and more text to use up space ⬅  More text to use up space ⬅  More Text to use up space ⬅  More text to use up space ⬅  More text to use up space ⬅  Final text section",
        "Banana",
        "Apple",
        "Strawberry",
        "Grapes",
        "Lemon",
        "Tangerine",
        "Watermelon",
        "Orange",
        "Pear",
        "Avocado",
        "Pineapple",
        //"Download - school webpage - https://web.archive.org/web/20150910082528/http://www.cs.utah.edu/~rchriste/  ⬅ 🎯🏞 Save a history of my life / Journaling / Family History ⬅ 🎯 Save a history of my life / Journaling",
//        "🔥 [👨‍👦 Needs a reason] |🔴 🪜 Some text to use up space  ⬅ 🪜 More text to use up space ⬅ 🪜 More and more and more text to use up space ⬅ 🪜 More and more text to use up space ⬅ 🪜 More and more text to use up space ⬅ 🪜 More text to use up space ⬅ 🪜 More Text to use up space ⬅ 🪜 More text to use up space ⬅ 🪜 More text to use up space ⬅ 🎯 Final text section",
// "🔥 [🏗️ Needs a reason] |🔴 🪜 Some text to use up space  ⬅ 🪜 More text to use up space ⬅ 🪜 More and more and more text to use up space ⬅ 🪜 More and more text to use up space ⬅ 🪜 More and more text to use up space ⬅ 🪜 More text to use up space ⬅ 🪜 More Text to use up space ⬅ 🪜 More text to use up space ⬅ 🪜 More text to use up space ⬅ 🎯 Final text section",
//This next one works fine
//"🔥 [🏃 Needs a reason] |🔴 🪜 Some text to use up space  ⬅ 🪜 More text to use up space ⬅ 🪜 More and more and more text to use up space ⬅ 🪜 More and more text to use up space ⬅ 🪜 More and more text to use up space ⬅ 🪜 More text to use up space ⬅ 🪜 More Text to use up space ⬅ 🪜 More text to use up space ⬅ 🪜 More text to use up space ⬅ 🎯 Final text section",
//         "🔥 [🏃‍♂️ Needs a reason] |🔴 🪜 Some text to use up space  ⬅ 🪜 More text to use up space ⬅ 🪜 More and more and more text to use up space ⬅ 🪜 More and more text to use up space ⬅ 🪜 More and more text to use up space ⬅ 🪜 More text to use up space ⬅ 🪜 More Text to use up space ⬅ 🪜 More text to use up space ⬅ 🪜 More text to use up space ⬅ 🎯 Final text section",

     ];

    let ans = Select::new("What's your favorite fruit?", options).prompt();

    match ans {
        Ok(choice) => println!("{choice}! That's mine too!"),
        Err(_) => println!("There was an error, please try again"),
    }
}
