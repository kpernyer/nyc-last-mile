//! Location name mapping for realistic display
//! Maps ZIP3 codes to short (3-letter) and long location names

use std::collections::HashMap;
use std::sync::LazyLock;

/// Location info: (short_name, long_name)
pub static LOCATION_NAMES: LazyLock<HashMap<&'static str, (&'static str, &'static str)>> = LazyLock::new(|| {
    let mut m = HashMap::new();

    // Northeast
    m.insert("100", ("NYC", "New York, NY"));
    m.insert("101", ("NYC", "New York, NY"));
    m.insert("102", ("NYC", "New York, NY"));
    m.insert("103", ("NYC", "Staten Island, NY"));
    m.insert("104", ("BRX", "Bronx, NY"));
    m.insert("110", ("QNS", "Queens, NY"));
    m.insert("111", ("LIC", "Long Island City, NY"));
    m.insert("112", ("BKN", "Brooklyn, NY"));
    m.insert("113", ("BKN", "Brooklyn, NY"));
    m.insert("114", ("JFK", "Jamaica, NY"));
    m.insert("115", ("WBY", "Westbury, NY"));
    m.insert("117", ("HVL", "Hicksville, NY"));
    m.insert("119", ("RVH", "Riverhead, NY"));
    m.insert("120", ("ALB", "Albany, NY"));
    m.insert("121", ("ALB", "Albany, NY"));
    m.insert("122", ("ALB", "Albany, NY"));
    m.insert("123", ("SCH", "Schenectady, NY"));
    m.insert("124", ("KGS", "Kingston, NY"));
    m.insert("125", ("PKP", "Poughkeepsie, NY"));
    m.insert("126", ("PKP", "Poughkeepsie, NY"));
    m.insert("127", ("MTC", "Monticello, NY"));
    m.insert("128", ("GLN", "Glens Falls, NY"));
    m.insert("129", ("PLT", "Plattsburgh, NY"));
    m.insert("130", ("SYR", "Syracuse, NY"));
    m.insert("131", ("SYR", "Syracuse, NY"));
    m.insert("132", ("SYR", "Syracuse, NY"));
    m.insert("133", ("UTI", "Utica, NY"));
    m.insert("134", ("UTI", "Utica, NY"));
    m.insert("135", ("UTI", "Utica, NY"));
    m.insert("136", ("WAT", "Watertown, NY"));
    m.insert("137", ("BNG", "Binghamton, NY"));
    m.insert("138", ("BNG", "Binghamton, NY"));
    m.insert("139", ("BNG", "Binghamton, NY"));
    m.insert("140", ("BUF", "Buffalo, NY"));
    m.insert("141", ("BUF", "Buffalo, NY"));
    m.insert("142", ("BUF", "Buffalo, NY"));
    m.insert("143", ("NIA", "Niagara Falls, NY"));
    m.insert("144", ("ROC", "Rochester, NY"));
    m.insert("145", ("ROC", "Rochester, NY"));
    m.insert("146", ("ROC", "Rochester, NY"));
    m.insert("147", ("JAM", "Jamestown, NY"));
    m.insert("148", ("ELM", "Elmira, NY"));
    m.insert("149", ("ELM", "Elmira, NY"));

    // Pennsylvania
    m.insert("150", ("PIT", "Pittsburgh, PA"));
    m.insert("151", ("PIT", "Pittsburgh, PA"));
    m.insert("152", ("PIT", "Pittsburgh, PA"));
    m.insert("153", ("PIT", "Pittsburgh, PA"));
    m.insert("154", ("PIT", "Pittsburgh, PA"));
    m.insert("155", ("JNT", "Johnstown, PA"));
    m.insert("156", ("GBG", "Greensburg, PA"));
    m.insert("157", ("IND", "Indiana, PA"));
    m.insert("158", ("DUB", "DuBois, PA"));
    m.insert("159", ("JNT", "Johnstown, PA"));
    m.insert("160", ("BTL", "Butler, PA"));
    m.insert("161", ("NBG", "New Castle, PA"));
    m.insert("162", ("NBG", "New Castle, PA"));
    m.insert("163", ("OCP", "Oil City, PA"));
    m.insert("164", ("ERI", "Erie, PA"));
    m.insert("165", ("ERI", "Erie, PA"));
    m.insert("166", ("ALT", "Altoona, PA"));
    m.insert("167", ("BFD", "Bradford, PA"));
    m.insert("168", ("STU", "State College, PA"));
    m.insert("169", ("WBR", "Wellsboro, PA"));
    m.insert("170", ("HBG", "Harrisburg, PA"));
    m.insert("171", ("HBG", "Harrisburg, PA"));
    m.insert("172", ("HBG", "Harrisburg, PA"));
    m.insert("173", ("YRK", "York, PA"));
    m.insert("174", ("YRK", "York, PA"));
    m.insert("175", ("LAN", "Lancaster, PA"));
    m.insert("176", ("LAN", "Lancaster, PA"));
    m.insert("177", ("WMS", "Williamsport, PA"));
    m.insert("178", ("SUN", "Sunbury, PA"));
    m.insert("179", ("PTT", "Pottsville, PA"));
    m.insert("180", ("LHV", "Lehigh Valley, PA"));
    m.insert("181", ("ALN", "Allentown, PA"));
    m.insert("182", ("HZN", "Hazleton, PA"));
    m.insert("183", ("LHV", "Lehigh Valley, PA"));
    m.insert("184", ("SCR", "Scranton, PA"));
    m.insert("185", ("SCR", "Scranton, PA"));
    m.insert("186", ("WKB", "Wilkes-Barre, PA"));
    m.insert("187", ("WKB", "Wilkes-Barre, PA"));
    m.insert("188", ("SCR", "Scranton, PA"));
    m.insert("189", ("DOY", "Doylestown, PA"));
    m.insert("190", ("PHL", "Philadelphia, PA"));
    m.insert("191", ("PHL", "Philadelphia, PA"));
    m.insert("192", ("PHL", "Philadelphia, PA"));
    m.insert("193", ("PHL", "Philadelphia, PA"));
    m.insert("194", ("NRS", "Norristown, PA"));
    m.insert("195", ("RDG", "Reading, PA"));
    m.insert("196", ("RDG", "Reading, PA"));

    // New Jersey
    m.insert("070", ("NWK", "Newark, NJ"));
    m.insert("071", ("NWK", "Newark, NJ"));
    m.insert("072", ("ELZ", "Elizabeth, NJ"));
    m.insert("073", ("JCY", "Jersey City, NJ"));
    m.insert("074", ("PAT", "Paterson, NJ"));
    m.insert("075", ("PAT", "Paterson, NJ"));
    m.insert("076", ("HCK", "Hackensack, NJ"));
    m.insert("077", ("RED", "Red Bank, NJ"));
    m.insert("078", ("RED", "Red Bank, NJ"));
    m.insert("079", ("SBK", "South Brunswick, NJ"));
    m.insert("080", ("CAM", "Camden, NJ"));
    m.insert("081", ("CAM", "Camden, NJ"));
    m.insert("082", ("CAM", "Camden, NJ"));
    m.insert("083", ("CAM", "Camden, NJ"));
    m.insert("084", ("ACY", "Atlantic City, NJ"));
    m.insert("085", ("TRT", "Trenton, NJ"));
    m.insert("086", ("TRT", "Trenton, NJ"));
    m.insert("087", ("TOM", "Toms River, NJ"));
    m.insert("088", ("NBK", "New Brunswick, NJ"));
    m.insert("089", ("NBK", "New Brunswick, NJ"));

    // Massachusetts
    m.insert("010", ("SPF", "Springfield, MA"));
    m.insert("011", ("SPF", "Springfield, MA"));
    m.insert("012", ("PIT", "Pittsfield, MA"));
    m.insert("013", ("SPF", "Springfield, MA"));
    m.insert("014", ("WOR", "Worcester, MA"));
    m.insert("015", ("WOR", "Worcester, MA"));
    m.insert("016", ("WOR", "Worcester, MA"));
    m.insert("017", ("FRM", "Framingham, MA"));
    m.insert("018", ("WOB", "Woburn, MA"));
    m.insert("019", ("LYN", "Lynn, MA"));
    m.insert("020", ("BOS", "Boston, MA"));
    m.insert("021", ("BOS", "Boston, MA"));
    m.insert("022", ("BOS", "Boston, MA"));
    m.insert("023", ("BRO", "Brockton, MA"));
    m.insert("024", ("BOS", "Boston, MA"));
    m.insert("025", ("CPE", "Cape Cod, MA"));
    m.insert("026", ("CPE", "Cape Cod, MA"));
    m.insert("027", ("NBD", "New Bedford, MA"));

    // Connecticut
    m.insert("060", ("HFD", "Hartford, CT"));
    m.insert("061", ("HFD", "Hartford, CT"));
    m.insert("062", ("HFD", "Hartford, CT"));
    m.insert("063", ("NHV", "New Haven, CT"));
    m.insert("064", ("NHV", "New Haven, CT"));
    m.insert("065", ("NHV", "New Haven, CT"));
    m.insert("066", ("BPT", "Bridgeport, CT"));
    m.insert("067", ("WBY", "Waterbury, CT"));
    m.insert("068", ("STM", "Stamford, CT"));
    m.insert("069", ("STM", "Stamford, CT"));

    // Maryland / DC / Virginia
    m.insert("200", ("DCA", "Washington, DC"));
    m.insert("201", ("DCA", "Washington, DC"));
    m.insert("202", ("DCA", "Washington, DC"));
    m.insert("203", ("DCA", "Washington, DC"));
    m.insert("204", ("DCA", "Washington, DC"));
    m.insert("205", ("DCA", "Washington, DC"));
    m.insert("206", ("SSP", "Suitland, MD"));
    m.insert("207", ("SSP", "Southern MD"));
    m.insert("208", ("SSP", "Suburban MD"));
    m.insert("209", ("SVS", "Silver Spring, MD"));
    m.insert("210", ("BAL", "Baltimore, MD"));
    m.insert("211", ("BAL", "Baltimore, MD"));
    m.insert("212", ("BAL", "Baltimore, MD"));
    m.insert("214", ("ANN", "Annapolis, MD"));
    m.insert("215", ("CUM", "Cumberland, MD"));
    m.insert("216", ("EST", "Easton, MD"));
    m.insert("217", ("FRD", "Frederick, MD"));
    m.insert("218", ("SBY", "Salisbury, MD"));
    m.insert("219", ("SBY", "Salisbury, MD"));
    m.insert("220", ("NOV", "Northern VA"));
    m.insert("221", ("NOV", "Northern VA"));
    m.insert("222", ("ARL", "Arlington, VA"));
    m.insert("223", ("ALX", "Alexandria, VA"));
    m.insert("224", ("STF", "Stafford, VA"));
    m.insert("225", ("FBG", "Fredericksburg, VA"));
    m.insert("226", ("WIN", "Winchester, VA"));
    m.insert("227", ("CUL", "Culpeper, VA"));
    m.insert("228", ("HAR", "Harrisonburg, VA"));
    m.insert("229", ("CHV", "Charlottesville, VA"));
    m.insert("230", ("RIC", "Richmond, VA"));
    m.insert("231", ("RIC", "Richmond, VA"));
    m.insert("232", ("RIC", "Richmond, VA"));
    m.insert("233", ("NFK", "Norfolk, VA"));
    m.insert("234", ("NFK", "Norfolk, VA"));
    m.insert("235", ("NFK", "Norfolk, VA"));
    m.insert("236", ("NFK", "Norfolk, VA"));
    m.insert("237", ("PTM", "Portsmouth, VA"));
    m.insert("238", ("RIC", "Richmond, VA"));
    m.insert("239", ("FMV", "Farmville, VA"));
    m.insert("240", ("ROA", "Roanoke, VA"));
    m.insert("241", ("ROA", "Roanoke, VA"));
    m.insert("242", ("BRS", "Bristol, VA"));
    m.insert("243", ("PUL", "Pulaski, VA"));
    m.insert("244", ("STN", "Staunton, VA"));
    m.insert("245", ("LYN", "Lynchburg, VA"));
    m.insert("246", ("BLU", "Bluefield, VA"));

    // North Carolina
    m.insert("270", ("GSO", "Greensboro, NC"));
    m.insert("271", ("WSL", "Winston-Salem, NC"));
    m.insert("272", ("GSO", "Greensboro, NC"));
    m.insert("273", ("GSO", "Greensboro, NC"));
    m.insert("274", ("GSO", "Greensboro, NC"));
    m.insert("275", ("RDU", "Raleigh, NC"));
    m.insert("276", ("RDU", "Raleigh, NC"));
    m.insert("277", ("RDU", "Durham, NC"));
    m.insert("278", ("RMT", "Rocky Mount, NC"));
    m.insert("279", ("RMT", "Rocky Mount, NC"));
    m.insert("280", ("CLT", "Charlotte, NC"));
    m.insert("281", ("CLT", "Charlotte, NC"));
    m.insert("282", ("CLT", "Charlotte, NC"));
    m.insert("283", ("FAY", "Fayetteville, NC"));
    m.insert("284", ("WLM", "Wilmington, NC"));
    m.insert("285", ("KNS", "Kinston, NC"));
    m.insert("286", ("HKP", "Hickory, NC"));
    m.insert("287", ("ASH", "Asheville, NC"));
    m.insert("288", ("ASH", "Asheville, NC"));
    m.insert("289", ("ASH", "Asheville, NC"));

    // South Carolina
    m.insert("290", ("CHS", "Charleston, SC"));
    m.insert("291", ("CHS", "Charleston, SC"));
    m.insert("292", ("COL", "Columbia, SC"));
    m.insert("293", ("COL", "Columbia, SC"));
    m.insert("294", ("CHS", "Charleston, SC"));
    m.insert("295", ("FLO", "Florence, SC"));
    m.insert("296", ("GVL", "Greenville, SC"));
    m.insert("297", ("GVL", "Greenville, SC"));
    m.insert("298", ("AUG", "Augusta, GA"));
    m.insert("299", ("SAV", "Savannah, GA"));

    // Georgia
    m.insert("300", ("ATL", "Atlanta, GA"));
    m.insert("301", ("ATL", "Atlanta, GA"));
    m.insert("302", ("ATL", "Atlanta, GA"));
    m.insert("303", ("ATL", "Atlanta, GA"));
    m.insert("304", ("STN", "Statesboro, GA"));
    m.insert("305", ("ATH", "Athens, GA"));
    m.insert("306", ("ATH", "Athens, GA"));
    m.insert("307", ("CHT", "Chattanooga, TN"));
    m.insert("308", ("AUG", "Augusta, GA"));
    m.insert("309", ("AUG", "Augusta, GA"));
    m.insert("310", ("MCN", "Macon, GA"));
    m.insert("311", ("ATL", "Atlanta, GA"));
    m.insert("312", ("MCN", "Macon, GA"));
    m.insert("313", ("SAV", "Savannah, GA"));
    m.insert("314", ("SAV", "Savannah, GA"));
    m.insert("315", ("WAY", "Waycross, GA"));
    m.insert("316", ("VLD", "Valdosta, GA"));
    m.insert("317", ("ALB", "Albany, GA"));
    m.insert("318", ("COL", "Columbus, GA"));
    m.insert("319", ("COL", "Columbus, GA"));

    // Florida
    m.insert("320", ("JAX", "Jacksonville, FL"));
    m.insert("321", ("DAY", "Daytona Beach, FL"));
    m.insert("322", ("JAX", "Jacksonville, FL"));
    m.insert("323", ("TLH", "Tallahassee, FL"));
    m.insert("324", ("PAN", "Panama City, FL"));
    m.insert("325", ("PNS", "Pensacola, FL"));
    m.insert("326", ("GNV", "Gainesville, FL"));
    m.insert("327", ("ORL", "Orlando, FL"));
    m.insert("328", ("ORL", "Orlando, FL"));
    m.insert("329", ("MLB", "Melbourne, FL"));
    m.insert("330", ("MIA", "Miami, FL"));
    m.insert("331", ("MIA", "Miami, FL"));
    m.insert("332", ("MIA", "Miami, FL"));
    m.insert("333", ("FLL", "Ft Lauderdale, FL"));
    m.insert("334", ("WPB", "West Palm Beach, FL"));
    m.insert("335", ("TPA", "Tampa, FL"));
    m.insert("336", ("TPA", "Tampa, FL"));
    m.insert("337", ("STP", "St Petersburg, FL"));
    m.insert("338", ("LKL", "Lakeland, FL"));
    m.insert("339", ("FMY", "Ft Myers, FL"));
    m.insert("340", ("FMY", "Ft Myers, FL"));
    m.insert("341", ("FMY", "Ft Myers, FL"));
    m.insert("342", ("MNT", "Manasota, FL"));

    // Ohio
    m.insert("430", ("COL", "Columbus, OH"));
    m.insert("431", ("COL", "Columbus, OH"));
    m.insert("432", ("COL", "Columbus, OH"));
    m.insert("433", ("COL", "Columbus, OH"));
    m.insert("434", ("COL", "Columbus, OH"));
    m.insert("435", ("COL", "Columbus, OH"));
    m.insert("436", ("TOL", "Toledo, OH"));
    m.insert("437", ("ZAN", "Zanesville, OH"));
    m.insert("438", ("ZAN", "Zanesville, OH"));
    m.insert("439", ("STB", "Steubenville, OH"));
    m.insert("440", ("CLE", "Cleveland, OH"));
    m.insert("441", ("CLE", "Cleveland, OH"));
    m.insert("442", ("AKR", "Akron, OH"));
    m.insert("443", ("AKR", "Akron, OH"));
    m.insert("444", ("YNG", "Youngstown, OH"));
    m.insert("445", ("YNG", "Youngstown, OH"));
    m.insert("446", ("CAN", "Canton, OH"));
    m.insert("447", ("CAN", "Canton, OH"));
    m.insert("448", ("MNS", "Mansfield, OH"));
    m.insert("449", ("MNS", "Mansfield, OH"));
    m.insert("450", ("CIN", "Cincinnati, OH"));
    m.insert("451", ("CIN", "Cincinnati, OH"));
    m.insert("452", ("CIN", "Cincinnati, OH"));
    m.insert("453", ("DAY", "Dayton, OH"));
    m.insert("454", ("DAY", "Dayton, OH"));
    m.insert("455", ("SPF", "Springfield, OH"));
    m.insert("456", ("CHI", "Chillicothe, OH"));
    m.insert("457", ("ATH", "Athens, OH"));
    m.insert("458", ("LIM", "Lima, OH"));

    // Michigan
    m.insert("480", ("DTW", "Detroit, MI"));
    m.insert("481", ("DTW", "Detroit, MI"));
    m.insert("482", ("DTW", "Detroit, MI"));
    m.insert("483", ("DTW", "Detroit, MI"));
    m.insert("484", ("FLN", "Flint, MI"));
    m.insert("485", ("FLN", "Flint, MI"));
    m.insert("486", ("SAG", "Saginaw, MI"));
    m.insert("487", ("SAG", "Saginaw, MI"));
    m.insert("488", ("LAN", "Lansing, MI"));
    m.insert("489", ("LAN", "Lansing, MI"));
    m.insert("490", ("KZO", "Kalamazoo, MI"));
    m.insert("491", ("KZO", "Kalamazoo, MI"));
    m.insert("492", ("JAK", "Jackson, MI"));
    m.insert("493", ("GRR", "Grand Rapids, MI"));
    m.insert("494", ("GRR", "Grand Rapids, MI"));
    m.insert("495", ("GRR", "Grand Rapids, MI"));
    m.insert("496", ("TVC", "Traverse City, MI"));
    m.insert("497", ("GYL", "Gaylord, MI"));
    m.insert("498", ("IRN", "Iron Mountain, MI"));
    m.insert("499", ("IRN", "Iron Mountain, MI"));

    // Indiana
    m.insert("460", ("IND", "Indianapolis, IN"));
    m.insert("461", ("IND", "Indianapolis, IN"));
    m.insert("462", ("IND", "Indianapolis, IN"));
    m.insert("463", ("GRY", "Gary, IN"));
    m.insert("464", ("GRY", "Gary, IN"));
    m.insert("465", ("SBN", "South Bend, IN"));
    m.insert("466", ("SBN", "South Bend, IN"));
    m.insert("467", ("FWA", "Fort Wayne, IN"));
    m.insert("468", ("FWA", "Fort Wayne, IN"));
    m.insert("469", ("KOK", "Kokomo, IN"));
    m.insert("470", ("CIN", "Cincinnati, OH"));
    m.insert("471", ("LOV", "Louisville, KY"));
    m.insert("472", ("COL", "Columbus, IN"));
    m.insert("473", ("MUN", "Muncie, IN"));
    m.insert("474", ("BLM", "Bloomington, IN"));
    m.insert("475", ("WAS", "Washington, IN"));
    m.insert("476", ("EVV", "Evansville, IN"));
    m.insert("477", ("EVV", "Evansville, IN"));
    m.insert("478", ("TRH", "Terre Haute, IN"));
    m.insert("479", ("LAF", "Lafayette, IN"));

    // Illinois
    m.insert("600", ("CHI", "Chicago, IL"));
    m.insert("601", ("CHI", "Chicago, IL"));
    m.insert("602", ("CHI", "Chicago, IL"));
    m.insert("603", ("CHI", "Chicago, IL"));
    m.insert("604", ("CHI", "Chicago, IL"));
    m.insert("605", ("CHI", "Chicago, IL"));
    m.insert("606", ("CHI", "Chicago, IL"));
    m.insert("607", ("CHI", "Chicago, IL"));
    m.insert("608", ("CHI", "Chicago, IL"));
    m.insert("609", ("KAN", "Kankakee, IL"));
    m.insert("610", ("RFD", "Rockford, IL"));
    m.insert("611", ("RFD", "Rockford, IL"));
    m.insert("612", ("RFD", "Rockford, IL"));
    m.insert("613", ("DIX", "Dixon, IL"));
    m.insert("614", ("GAL", "Galesburg, IL"));
    m.insert("615", ("PEO", "Peoria, IL"));
    m.insert("616", ("PEO", "Peoria, IL"));
    m.insert("617", ("BLM", "Bloomington, IL"));
    m.insert("618", ("BLM", "Bloomington, IL"));
    m.insert("619", ("BLM", "Bloomington, IL"));
    m.insert("620", ("STL", "St. Louis, MO"));
    m.insert("622", ("STL", "St. Louis, MO"));
    m.insert("623", ("QCY", "Quincy, IL"));
    m.insert("624", ("EFF", "Effingham, IL"));
    m.insert("625", ("SPF", "Springfield, IL"));
    m.insert("626", ("SPF", "Springfield, IL"));
    m.insert("627", ("SPF", "Springfield, IL"));
    m.insert("628", ("CEN", "Centralia, IL"));
    m.insert("629", ("CAR", "Carbondale, IL"));

    // Wisconsin
    m.insert("530", ("MKE", "Milwaukee, WI"));
    m.insert("531", ("MKE", "Milwaukee, WI"));
    m.insert("532", ("MKE", "Milwaukee, WI"));
    m.insert("534", ("RAC", "Racine, WI"));
    m.insert("535", ("MSN", "Madison, WI"));
    m.insert("537", ("MSN", "Madison, WI"));
    m.insert("538", ("MSN", "Madison, WI"));
    m.insert("539", ("PRT", "Portage, WI"));
    m.insert("540", ("STN", "Stevens Point, WI"));
    m.insert("541", ("GRB", "Green Bay, WI"));
    m.insert("542", ("GRB", "Green Bay, WI"));
    m.insert("543", ("GRB", "Green Bay, WI"));
    m.insert("544", ("WAU", "Wausau, WI"));
    m.insert("545", ("RHN", "Rhinelander, WI"));
    m.insert("546", ("LAC", "La Crosse, WI"));
    m.insert("547", ("EAU", "Eau Claire, WI"));
    m.insert("548", ("SPR", "Superior, WI"));
    m.insert("549", ("OSH", "Oshkosh, WI"));

    // Minnesota
    m.insert("550", ("MSP", "Minneapolis, MN"));
    m.insert("551", ("MSP", "St. Paul, MN"));
    m.insert("553", ("MSP", "Minneapolis, MN"));
    m.insert("554", ("MSP", "Minneapolis, MN"));
    m.insert("555", ("MSP", "Minneapolis, MN"));
    m.insert("556", ("DLH", "Duluth, MN"));
    m.insert("557", ("DLH", "Duluth, MN"));
    m.insert("558", ("DLH", "Duluth, MN"));
    m.insert("559", ("ROC", "Rochester, MN"));
    m.insert("560", ("MKT", "Mankato, MN"));
    m.insert("561", ("MKT", "Mankato, MN"));
    m.insert("562", ("WLR", "Willmar, MN"));
    m.insert("563", ("STD", "St. Cloud, MN"));
    m.insert("564", ("BRD", "Brainerd, MN"));
    m.insert("565", ("DET", "Detroit Lakes, MN"));
    m.insert("566", ("BMI", "Bemidji, MN"));
    m.insert("567", ("TRF", "Thief River Falls, MN"));

    // Iowa
    m.insert("500", ("DSM", "Des Moines, IA"));
    m.insert("501", ("DSM", "Des Moines, IA"));
    m.insert("502", ("DSM", "Des Moines, IA"));
    m.insert("503", ("DSM", "Des Moines, IA"));
    m.insert("504", ("MSC", "Mason City, IA"));
    m.insert("505", ("FTD", "Fort Dodge, IA"));
    m.insert("506", ("WAT", "Waterloo, IA"));
    m.insert("507", ("WAT", "Waterloo, IA"));
    m.insert("508", ("CRR", "Creston, IA"));
    m.insert("509", ("DSM", "Des Moines, IA"));
    m.insert("510", ("SXC", "Sioux City, IA"));
    m.insert("511", ("SXC", "Sioux City, IA"));
    m.insert("512", ("SHY", "Sheldon, IA"));
    m.insert("513", ("SPT", "Spencer, IA"));
    m.insert("514", ("CRL", "Carroll, IA"));
    m.insert("515", ("OMA", "Omaha, NE"));
    m.insert("516", ("SHN", "Shenandoah, IA"));
    m.insert("520", ("DVP", "Davenport, IA"));
    m.insert("521", ("DVP", "Davenport, IA"));
    m.insert("522", ("CDR", "Cedar Rapids, IA"));
    m.insert("523", ("CDR", "Cedar Rapids, IA"));
    m.insert("524", ("CDR", "Cedar Rapids, IA"));
    m.insert("525", ("OTW", "Ottumwa, IA"));
    m.insert("526", ("BRL", "Burlington, IA"));
    m.insert("527", ("DBQ", "Dubuque, IA"));
    m.insert("528", ("DBQ", "Dubuque, IA"));

    // Missouri
    m.insert("630", ("STL", "St. Louis, MO"));
    m.insert("631", ("STL", "St. Louis, MO"));
    m.insert("633", ("STL", "St. Louis, MO"));
    m.insert("634", ("QCY", "Quincy, IL"));
    m.insert("635", ("QCY", "Quincy, IL"));
    m.insert("636", ("CPG", "Cape Girardeau, MO"));
    m.insert("637", ("CPG", "Cape Girardeau, MO"));
    m.insert("638", ("CPG", "Cape Girardeau, MO"));
    m.insert("639", ("POP", "Poplar Bluff, MO"));
    m.insert("640", ("MCI", "Kansas City, MO"));
    m.insert("641", ("MCI", "Kansas City, MO"));
    m.insert("644", ("SPG", "Springfield, MO"));
    m.insert("645", ("SPG", "Springfield, MO"));
    m.insert("646", ("CHI", "Chillicothe, MO"));
    m.insert("647", ("HBL", "Harrisonville, MO"));
    m.insert("648", ("JOP", "Joplin, MO"));
    m.insert("649", ("MCI", "Kansas City, MO"));
    m.insert("650", ("JEF", "Jefferson City, MO"));
    m.insert("651", ("JEF", "Jefferson City, MO"));
    m.insert("652", ("COL", "Columbia, MO"));
    m.insert("653", ("SED", "Sedalia, MO"));
    m.insert("654", ("ROL", "Rolla, MO"));
    m.insert("655", ("ROL", "Rolla, MO"));
    m.insert("656", ("SPF", "Springfield, MO"));
    m.insert("657", ("SPF", "Springfield, MO"));
    m.insert("658", ("SPF", "Springfield, MO"));

    // Kansas
    m.insert("660", ("MCI", "Kansas City, KS"));
    m.insert("661", ("MCI", "Kansas City, KS"));
    m.insert("662", ("MCI", "Kansas City, KS"));
    m.insert("664", ("TOP", "Topeka, KS"));
    m.insert("665", ("TOP", "Topeka, KS"));
    m.insert("666", ("TOP", "Topeka, KS"));
    m.insert("667", ("FTS", "Fort Scott, KS"));
    m.insert("668", ("TOP", "Topeka, KS"));
    m.insert("669", ("BEL", "Belleville, KS"));
    m.insert("670", ("ICT", "Wichita, KS"));
    m.insert("671", ("ICT", "Wichita, KS"));
    m.insert("672", ("ICT", "Wichita, KS"));
    m.insert("673", ("IND", "Independence, KS"));
    m.insert("674", ("SAL", "Salina, KS"));
    m.insert("675", ("HUT", "Hutchinson, KS"));
    m.insert("676", ("HAY", "Hays, KS"));
    m.insert("677", ("COL", "Colby, KS"));
    m.insert("678", ("DOD", "Dodge City, KS"));
    m.insert("679", ("LIB", "Liberal, KS"));

    // Nebraska
    m.insert("680", ("OMA", "Omaha, NE"));
    m.insert("681", ("OMA", "Omaha, NE"));
    m.insert("683", ("LNK", "Lincoln, NE"));
    m.insert("684", ("LNK", "Lincoln, NE"));
    m.insert("685", ("LNK", "Lincoln, NE"));
    m.insert("686", ("NFK", "Norfolk, NE"));
    m.insert("687", ("NFK", "Norfolk, NE"));
    m.insert("688", ("GRI", "Grand Island, NE"));
    m.insert("689", ("GRI", "Grand Island, NE"));
    m.insert("690", ("MCC", "McCook, NE"));
    m.insert("691", ("NPI", "North Platte, NE"));
    m.insert("692", ("NPI", "North Platte, NE"));
    m.insert("693", ("ALI", "Alliance, NE"));

    // Texas
    m.insert("750", ("DFW", "Dallas, TX"));
    m.insert("751", ("DFW", "Dallas, TX"));
    m.insert("752", ("DFW", "Dallas, TX"));
    m.insert("753", ("DFW", "Dallas, TX"));
    m.insert("754", ("DEN", "Denton, TX"));
    m.insert("755", ("TXK", "Texarkana, TX"));
    m.insert("756", ("LGV", "Longview, TX"));
    m.insert("757", ("TYL", "Tyler, TX"));
    m.insert("758", ("PLT", "Palestine, TX"));
    m.insert("759", ("LFK", "Lufkin, TX"));
    m.insert("760", ("FTW", "Fort Worth, TX"));
    m.insert("761", ("FTW", "Fort Worth, TX"));
    m.insert("762", ("FTW", "Fort Worth, TX"));
    m.insert("763", ("WIC", "Wichita Falls, TX"));
    m.insert("764", ("WIC", "Wichita Falls, TX"));
    m.insert("765", ("WCO", "Waco, TX"));
    m.insert("766", ("WCO", "Waco, TX"));
    m.insert("767", ("WCO", "Waco, TX"));
    m.insert("768", ("ABL", "Abilene, TX"));
    m.insert("769", ("MID", "Midland, TX"));
    m.insert("770", ("HOU", "Houston, TX"));
    m.insert("771", ("HOU", "Houston, TX"));
    m.insert("772", ("HOU", "Houston, TX"));
    m.insert("773", ("HOU", "Houston, TX"));
    m.insert("774", ("HOU", "Houston, TX"));
    m.insert("775", ("HOU", "Houston, TX"));
    m.insert("776", ("BMT", "Beaumont, TX"));
    m.insert("777", ("BMT", "Beaumont, TX"));
    m.insert("778", ("BYN", "Bryan, TX"));
    m.insert("779", ("VIC", "Victoria, TX"));
    m.insert("780", ("SAT", "San Antonio, TX"));
    m.insert("781", ("SAT", "San Antonio, TX"));
    m.insert("782", ("SAT", "San Antonio, TX"));
    m.insert("783", ("CRP", "Corpus Christi, TX"));
    m.insert("784", ("CRP", "Corpus Christi, TX"));
    m.insert("785", ("MCA", "McAllen, TX"));
    m.insert("786", ("AUS", "Austin, TX"));
    m.insert("787", ("AUS", "Austin, TX"));
    m.insert("788", ("UVD", "Uvalde, TX"));
    m.insert("789", ("GGG", "Giddings, TX"));
    m.insert("790", ("AMA", "Amarillo, TX"));
    m.insert("791", ("AMA", "Amarillo, TX"));
    m.insert("792", ("AMA", "Childress, TX"));
    m.insert("793", ("LBB", "Lubbock, TX"));
    m.insert("794", ("LBB", "Lubbock, TX"));
    m.insert("795", ("LBB", "Lubbock, TX"));
    m.insert("796", ("LBB", "Lubbock, TX"));
    m.insert("797", ("MID", "Midland, TX"));
    m.insert("798", ("ELP", "El Paso, TX"));
    m.insert("799", ("ELP", "El Paso, TX"));

    // Oklahoma
    m.insert("730", ("OKC", "Oklahoma City, OK"));
    m.insert("731", ("OKC", "Oklahoma City, OK"));
    m.insert("734", ("ADM", "Ardmore, OK"));
    m.insert("735", ("LAW", "Lawton, OK"));
    m.insert("736", ("CLI", "Clinton, OK"));
    m.insert("737", ("ENI", "Enid, OK"));
    m.insert("738", ("WDW", "Woodward, OK"));
    m.insert("739", ("LIB", "Liberal, KS"));
    m.insert("740", ("TUL", "Tulsa, OK"));
    m.insert("741", ("TUL", "Tulsa, OK"));
    m.insert("743", ("MIA", "Miami, OK"));
    m.insert("744", ("MUS", "Muskogee, OK"));
    m.insert("745", ("MCA", "McAlester, OK"));
    m.insert("746", ("PON", "Ponca City, OK"));
    m.insert("747", ("DUR", "Durant, OK"));
    m.insert("748", ("SHW", "Shawnee, OK"));
    m.insert("749", ("POT", "Poteau, OK"));

    // Arkansas
    m.insert("716", ("LIT", "Little Rock, AR"));
    m.insert("717", ("CAM", "Camden, AR"));
    m.insert("718", ("TXK", "Texarkana, AR"));
    m.insert("719", ("HPS", "Hot Springs, AR"));
    m.insert("720", ("LIT", "Little Rock, AR"));
    m.insert("721", ("LIT", "Little Rock, AR"));
    m.insert("722", ("LIT", "Little Rock, AR"));
    m.insert("723", ("WMB", "West Memphis, AR"));
    m.insert("724", ("JBR", "Jonesboro, AR"));
    m.insert("725", ("BAT", "Batesville, AR"));
    m.insert("726", ("HAR", "Harrison, AR"));
    m.insert("727", ("FAY", "Fayetteville, AR"));
    m.insert("728", ("RUS", "Russellville, AR"));
    m.insert("729", ("FSM", "Fort Smith, AR"));

    // Louisiana
    m.insert("700", ("MSY", "New Orleans, LA"));
    m.insert("701", ("MSY", "New Orleans, LA"));
    m.insert("703", ("THB", "Thibodaux, LA"));
    m.insert("704", ("HMA", "Hammond, LA"));
    m.insert("705", ("LAF", "Lafayette, LA"));
    m.insert("706", ("LKC", "Lake Charles, LA"));
    m.insert("707", ("BTR", "Baton Rouge, LA"));
    m.insert("708", ("BTR", "Baton Rouge, LA"));
    m.insert("710", ("SHV", "Shreveport, LA"));
    m.insert("711", ("SHV", "Shreveport, LA"));
    m.insert("712", ("MON", "Monroe, LA"));
    m.insert("713", ("AXA", "Alexandria, LA"));
    m.insert("714", ("AXA", "Alexandria, LA"));

    // Tennessee
    m.insert("370", ("NSH", "Nashville, TN"));
    m.insert("371", ("NSH", "Nashville, TN"));
    m.insert("372", ("NSH", "Nashville, TN"));
    m.insert("373", ("CHT", "Chattanooga, TN"));
    m.insert("374", ("CHT", "Chattanooga, TN"));
    m.insert("376", ("JOH", "Johnson City, TN"));
    m.insert("377", ("KNX", "Knoxville, TN"));
    m.insert("378", ("KNX", "Knoxville, TN"));
    m.insert("379", ("KNX", "Knoxville, TN"));
    m.insert("380", ("MEM", "Memphis, TN"));
    m.insert("381", ("MEM", "Memphis, TN"));
    m.insert("382", ("MCK", "McKenzie, TN"));
    m.insert("383", ("JAK", "Jackson, TN"));
    m.insert("384", ("COL", "Columbia, TN"));
    m.insert("385", ("CKV", "Cookeville, TN"));

    // Kentucky
    m.insert("400", ("SDF", "Louisville, KY"));
    m.insert("401", ("SDF", "Louisville, KY"));
    m.insert("402", ("SDF", "Louisville, KY"));
    m.insert("403", ("LEX", "Lexington, KY"));
    m.insert("404", ("LEX", "Lexington, KY"));
    m.insert("405", ("LEX", "Lexington, KY"));
    m.insert("406", ("FRK", "Frankfort, KY"));
    m.insert("407", ("CRB", "Corbin, KY"));
    m.insert("408", ("CRB", "Corbin, KY"));
    m.insert("409", ("CRB", "Corbin, KY"));
    m.insert("410", ("CIN", "Cincinnati, OH"));
    m.insert("411", ("ASH", "Ashland, KY"));
    m.insert("412", ("ASH", "Ashland, KY"));
    m.insert("413", ("CAM", "Campbellsville, KY"));
    m.insert("414", ("CAM", "Campbellsville, KY"));
    m.insert("415", ("PKV", "Pikeville, KY"));
    m.insert("416", ("PKV", "Pikeville, KY"));
    m.insert("417", ("HZD", "Hazard, KY"));
    m.insert("418", ("HZD", "Hazard, KY"));
    m.insert("420", ("PAD", "Paducah, KY"));
    m.insert("421", ("BGN", "Bowling Green, KY"));
    m.insert("422", ("BGN", "Bowling Green, KY"));
    m.insert("423", ("OWN", "Owensboro, KY"));
    m.insert("424", ("HND", "Henderson, KY"));
    m.insert("425", ("SOM", "Somerset, KY"));
    m.insert("426", ("SOM", "Somerset, KY"));
    m.insert("427", ("ELZ", "Elizabethtown, KY"));

    // California
    m.insert("900", ("LAX", "Los Angeles, CA"));
    m.insert("901", ("LAX", "Los Angeles, CA"));
    m.insert("902", ("ING", "Inglewood, CA"));
    m.insert("903", ("ING", "Inglewood, CA"));
    m.insert("904", ("SMO", "Santa Monica, CA"));
    m.insert("905", ("TOR", "Torrance, CA"));
    m.insert("906", ("LBH", "Long Beach, CA"));
    m.insert("907", ("LBH", "Long Beach, CA"));
    m.insert("908", ("LBH", "Long Beach, CA"));
    m.insert("910", ("PAS", "Pasadena, CA"));
    m.insert("911", ("PAS", "Pasadena, CA"));
    m.insert("912", ("GLN", "Glendale, CA"));
    m.insert("913", ("BUR", "Burbank, CA"));
    m.insert("914", ("VNY", "Van Nuys, CA"));
    m.insert("915", ("BUR", "Burbank, CA"));
    m.insert("916", ("SFV", "San Fernando, CA"));
    m.insert("917", ("ARC", "Arcadia, CA"));
    m.insert("918", ("ARC", "Alhambra, CA"));
    m.insert("919", ("SDO", "San Dimas, CA"));
    m.insert("920", ("SDO", "San Dimas, CA"));
    m.insert("921", ("SNI", "San Bernardino, CA"));
    m.insert("922", ("SNI", "San Bernardino, CA"));
    m.insert("923", ("SNI", "San Bernardino, CA"));
    m.insert("924", ("SNI", "San Bernardino, CA"));
    m.insert("925", ("OAK", "Oakland, CA"));
    m.insert("926", ("SNA", "Santa Ana, CA"));
    m.insert("927", ("SNA", "Santa Ana, CA"));
    m.insert("928", ("ANA", "Anaheim, CA"));
    m.insert("930", ("VEN", "Ventura, CA"));
    m.insert("931", ("SBA", "Santa Barbara, CA"));
    m.insert("932", ("BAK", "Bakersfield, CA"));
    m.insert("933", ("BAK", "Bakersfield, CA"));
    m.insert("934", ("SLO", "San Luis Obispo, CA"));
    m.insert("935", ("MOJ", "Mojave, CA"));
    m.insert("936", ("FRE", "Fresno, CA"));
    m.insert("937", ("FRE", "Fresno, CA"));
    m.insert("939", ("SAL", "Salinas, CA"));
    m.insert("940", ("SFO", "San Francisco, CA"));
    m.insert("941", ("SFO", "San Francisco, CA"));
    m.insert("942", ("SAC", "Sacramento, CA"));
    m.insert("943", ("PAL", "Palo Alto, CA"));
    m.insert("944", ("SJC", "San Jose, CA"));
    m.insert("945", ("OAK", "Oakland, CA"));
    m.insert("946", ("OAK", "Oakland, CA"));
    m.insert("947", ("BRK", "Berkeley, CA"));
    m.insert("948", ("RCH", "Richmond, CA"));
    m.insert("949", ("SRF", "San Rafael, CA"));
    m.insert("950", ("SJC", "San Jose, CA"));
    m.insert("951", ("SJC", "San Jose, CA"));
    m.insert("952", ("STK", "Stockton, CA"));
    m.insert("953", ("STK", "Stockton, CA"));
    m.insert("954", ("SRO", "Santa Rosa, CA"));
    m.insert("955", ("EKA", "Eureka, CA"));
    m.insert("956", ("SAC", "Sacramento, CA"));
    m.insert("957", ("SAC", "Sacramento, CA"));
    m.insert("958", ("SAC", "Sacramento, CA"));
    m.insert("959", ("MAR", "Marysville, CA"));
    m.insert("960", ("RED", "Redding, CA"));
    m.insert("961", ("REN", "Reno, NV"));

    // San Diego
    m.insert("919", ("SAN", "San Diego, CA"));
    m.insert("920", ("SAN", "San Diego, CA"));
    m.insert("921", ("SAN", "San Diego, CA"));

    // Arizona
    m.insert("850", ("PHX", "Phoenix, AZ"));
    m.insert("851", ("PHX", "Phoenix, AZ"));
    m.insert("852", ("PHX", "Phoenix, AZ"));
    m.insert("853", ("PHX", "Phoenix, AZ"));
    m.insert("855", ("GLB", "Globe, AZ"));
    m.insert("856", ("TUS", "Tucson, AZ"));
    m.insert("857", ("TUS", "Tucson, AZ"));
    m.insert("859", ("SHW", "Show Low, AZ"));
    m.insert("860", ("FLG", "Flagstaff, AZ"));
    m.insert("863", ("PRS", "Prescott, AZ"));
    m.insert("864", ("KNG", "Kingman, AZ"));
    m.insert("865", ("GLL", "Gallup, NM"));

    // Nevada
    m.insert("889", ("LAS", "Las Vegas, NV"));
    m.insert("890", ("LAS", "Las Vegas, NV"));
    m.insert("891", ("LAS", "Las Vegas, NV"));
    m.insert("893", ("ELY", "Ely, NV"));
    m.insert("894", ("REN", "Reno, NV"));
    m.insert("895", ("REN", "Reno, NV"));
    m.insert("897", ("CRS", "Carson City, NV"));
    m.insert("898", ("ELK", "Elko, NV"));

    // Colorado
    m.insert("800", ("DEN", "Denver, CO"));
    m.insert("801", ("DEN", "Denver, CO"));
    m.insert("802", ("DEN", "Denver, CO"));
    m.insert("803", ("BOU", "Boulder, CO"));
    m.insert("804", ("DEN", "Denver, CO"));
    m.insert("805", ("LGM", "Longmont, CO"));
    m.insert("806", ("GRE", "Greeley, CO"));
    m.insert("807", ("FTC", "Fort Collins, CO"));
    m.insert("808", ("COS", "Colorado Springs, CO"));
    m.insert("809", ("COS", "Colorado Springs, CO"));
    m.insert("810", ("COS", "Colorado Springs, CO"));
    m.insert("811", ("ALM", "Alamosa, CO"));
    m.insert("812", ("SAL", "Salida, CO"));
    m.insert("813", ("DUR", "Durango, CO"));
    m.insert("814", ("GJT", "Grand Junction, CO"));
    m.insert("815", ("GJT", "Grand Junction, CO"));
    m.insert("816", ("GNW", "Glenwood Springs, CO"));

    // Utah
    m.insert("840", ("SLC", "Salt Lake City, UT"));
    m.insert("841", ("SLC", "Salt Lake City, UT"));
    m.insert("842", ("OGD", "Ogden, UT"));
    m.insert("843", ("LOG", "Logan, UT"));
    m.insert("844", ("OGD", "Ogden, UT"));
    m.insert("845", ("PRO", "Provo, UT"));
    m.insert("846", ("PRO", "Provo, UT"));
    m.insert("847", ("PRO", "Provo, UT"));

    // Oregon
    m.insert("970", ("PDX", "Portland, OR"));
    m.insert("971", ("PDX", "Portland, OR"));
    m.insert("972", ("PDX", "Portland, OR"));
    m.insert("973", ("SAL", "Salem, OR"));
    m.insert("974", ("EUG", "Eugene, OR"));
    m.insert("975", ("MED", "Medford, OR"));
    m.insert("976", ("KLF", "Klamath Falls, OR"));
    m.insert("977", ("BND", "Bend, OR"));
    m.insert("978", ("PND", "Pendleton, OR"));
    m.insert("979", ("BOI", "Boise, ID"));

    // Washington
    m.insert("980", ("SEA", "Seattle, WA"));
    m.insert("981", ("SEA", "Seattle, WA"));
    m.insert("982", ("EVT", "Everett, WA"));
    m.insert("983", ("TAC", "Tacoma, WA"));
    m.insert("984", ("TAC", "Tacoma, WA"));
    m.insert("985", ("OLY", "Olympia, WA"));
    m.insert("986", ("PDX", "Portland, OR"));
    m.insert("988", ("WEN", "Wenatchee, WA"));
    m.insert("989", ("YAK", "Yakima, WA"));
    m.insert("990", ("SPK", "Spokane, WA"));
    m.insert("991", ("SPK", "Spokane, WA"));
    m.insert("992", ("SPK", "Spokane, WA"));
    m.insert("993", ("PAS", "Pasco, WA"));
    m.insert("994", ("LEW", "Lewiston, ID"));

    m
});

/// Get short location name (3-letter code) for table display
pub fn get_location_short(zip3: &str) -> String {
    // Strip "xx" suffix if present
    let code = zip3.trim_end_matches("xx");

    LOCATION_NAMES
        .get(code)
        .map(|(short, _)| short.to_string())
        .unwrap_or_else(|| format!("{}", code))
}

/// Get long location name for detailed display
pub fn get_location_long(zip3: &str) -> String {
    let code = zip3.trim_end_matches("xx");

    LOCATION_NAMES
        .get(code)
        .map(|(_, long)| long.to_string())
        .unwrap_or_else(|| format!("ZIP {}", zip3))
}

/// Format a lane with short names (e.g., "DFW→AUS")
pub fn format_lane_short(origin: &str, dest: &str) -> String {
    format!("{}→{}", get_location_short(origin), get_location_short(dest))
}

/// Format a lane with long names (e.g., "Dallas, TX → Austin, TX")
pub fn format_lane_long(origin: &str, dest: &str) -> String {
    format!("{} → {}", get_location_long(origin), get_location_long(dest))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_short_name() {
        assert_eq!(get_location_short("750xx"), "DFW");
        assert_eq!(get_location_short("786xx"), "AUS");
        assert_eq!(get_location_short("150"), "PIT");
    }

    #[test]
    fn test_long_name() {
        assert_eq!(get_location_long("750xx"), "Dallas, TX");
        assert_eq!(get_location_long("786xx"), "Austin, TX");
    }

    #[test]
    fn test_lane_formatting() {
        assert_eq!(format_lane_short("750xx", "786xx"), "DFW→AUS");
        assert_eq!(format_lane_long("750xx", "786xx"), "Dallas, TX → Austin, TX");
    }

    #[test]
    fn test_unknown_location() {
        let short = get_location_short("999xx");
        assert_eq!(short, "999");

        let long = get_location_long("999xx");
        assert_eq!(long, "ZIP 999xx");
    }
}
