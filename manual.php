<!DOCTYPE html>
<html lang="en">
<link rel="stylesheet" href="styles.css?v=1.1">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <meta name="description" content="SMDB Companion - User Manual">
    <title>SMDB Companion - User Manual</title>
    <link rel="stylesheet" href="styles.css">   
</head>
<body>
    <?php include 'header.php'; ?>

    <!-- Manual Content Section -->
    <div class="manual-container">
        <!-- Sidebar Navigation -->
        <div class="sidebar">
            <h3>Contents</h3>
            <ul>
                <li><a href="#introduction" onclick="showContent('introduction')">Introduction</a></li>
                <li><a href="#installation" onclick="showContent('installation')">Installation</a></li>
                <li><a href="#registration" onclick="showContent('registration')">Registration</a></li>
                <li><a href="#getting-started" onclick="showContent('getting-started')">Getting Started</a></li>
                <li><a href="#features" onclick="showContent('features')">Features and Functionalities</a></li>
                <li><a href="#advanced-usage" onclick="showContent('advanced-usage')">Advanced Usage</a></li>
                <li><a href="#tips" onclick="showContent('tips')">Tips and Tricks</a></li>
                <li><a href="#contact" onclick="showContent('contact')">Contact/Support Information</a></li>
            </ul>
        </div>

        <!-- Content Area -->
        <div class="manual-content">
            <!-- Introduction Section -->
            <div id="introduction" class="manual-section">
                <h2>Introduction</h2>
                <img src="/smdbclogo.png" alt="SMDB Companion Logo" class="logo">
                <p>Welcome to the SMDB Companion user manual! This software helps you find and remove duplicate audio files from your Soundminer databases. This manual will guide you through the installation process, setup, usage, and advanced features of the application.</p>
               
            </div>

            <!-- Installation Section -->
            <div id="installation" class="manual-section">
                <h2>Installation and Setup</h2>
                <p>Follow the steps below to install and set up SMDB Companion:</p>
                <ol>
                    <li>Download the latest version of SMDB Companion from our <a href="https://smdbc.com/download.php">download page</a>.</li>
                    <li>Unzip the downloaded file and place in your Applications directory on your computer.</li>
                    <li>Ensure that you have Soundminer installed, as SMDB Companion requires it to function properly.</li>
                    <li>Ensure that <strong>System Settings -> Privacy & Security</strong> is set to allow: "App Store and identified developers"</li>
                    <li>Run the application by double-clicking the executable file (SMDB_Companion).</li>
                </ol>
            </div>

            <!-- Registration Section -->
            <div id="registration" class="manual-section">
                <h2>Registration</h2>
                <p> You will need to enter your registration code to remove duplicates from your database.  If you do not have a registration code, you can purchase one from our website. After purchase, your registration information will be sent to you via the email you use for purchasing.</p>
                <p> The registration menu can be found under the "File" menu in the top left.  Enter your name, email, and license key and click "Validate" to register.</p>
                <p><strong>NOTE:</strong> The registration code is CASE SENSITIVE.  Please copy and paste the code from the email to avoid any errors.</p>
                <img src="/assets/register.jpg">
                <p> Upon successful registration, you will see your registered name in the bottom of the application.</p>
                <?php include 'license-recovery.php'; ?>

            </div>

            <!-- Getting Started Section -->
            <div id="getting-started" class="manual-section">
                <h2>Getting Started</h2>
                <p>After installing SMDB Companion, it <strong>*strongly recommended*</strong> you back up your Soundminer databases before running the program.</p>
                <ol>
                    <li>Launch SMDB Companion and click "Open Database" to choose the Soundminer database you want to scan.</li>
                    <img src="assets/open_database.jpg">
                    <li>Choose which duplicate search algorithms you wish to enable and adjust any options. Hovering your mouse over an algorithm with reveal a tooltip with a brief description of what it does.</li>
                    <li>Click "Search for Duplicates" to begin finding duplicate audio files in your database.</li>
                    <img src="/assets/search_buttons2.jpg">
                    <li>If duplicates are found, a button for "Remove Duplicates" will be enabled.  Click to Remove. </li>
                    <li>If you have registered, you will also have to option to see your search results.</li>
                    <!-- <img src="/assets/full_view.jpg"> -->
                </ol>
                <p> For more detailed information on how to use the program, please refer to the "Features and Functionalities" section below.</p>
           
            </div>

            <!-- Features and Functionalities Section -->
            <div id="features" class="manual-section">
                <h2>Features and Functionalities</h2>
                <p>SMDB Companion now offers enhanced ways to work with your Soundminer Databases:</p>
                <ul>
                    <li><strong>Duplicate Search and Removal:</strong> Find and remove duplicate audio files using advanced algorithms.</li>
                    <li><strong>Metadata Find and Replace:</strong> Search for metadata in your database and replace it with new values, with added case sensitivity and column-specific options.</li>
                </ul>
                <p>Choose your desired feature under the "Action" Menu.</p>

                <h2>Duplicate Search</h2>
                <p><strong>New and Improved Search Algorithms:</strong></p>
                <ul>
                    <li><strong>Basic Search:</strong> Matches files based on criteria like File Name, Duration, and Channels.</li>
                    <li><strong>Similar Filename:</strong> Identifies files with similar root names, ignoring extensions.</li>
                    <li><strong>Tags:</strong> Detects files with specific tags, such as those added by Protools AudioSuite.</li>
                    <li><strong>Audio Length:</strong> Filters files shorter than a specified duration.</li>
                    <li><strong>Audio Content:</strong> Compares audio waveforms to find duplicates with different names.</li>
                    <li><strong>Database Compare:</strong> Compares the current database with another to identify duplicates.</li>
                    <li><strong>Dual Mono Detection:</strong> Identifies files with identical audio channels and offers to merge them.</li>
                </ul>

                <h2>Metadata Find and Replace</h2>
                <p>Enhanced metadata replacement capabilities include:</p>
                <ul>
                    <li>Case-sensitive or case-insensitive search and replace.</li>
                    <li>Column-specific operations for precise updates.</li>
                    <li>Marking records as "dirty" for easy tracking in Soundminer.</li>
                </ul>
                <p>Access this feature under the "Metadata" tab and configure your preferences.</p>

                <h2>Advanced Usage</h2>
                <p>Customize file preservation logic under <strong>"Preferences -> File Preservation Priority"</strong>:</p>
                <ul>
                    <li><strong>Top Toolbar:</strong> Add new rules to the list.</li>
                    <li><strong>Rules List:</strong> Prioritize rules for duplicate resolution.</li>
                    <li><strong>Bottom Toolbar:</strong> Reorder or remove rules as needed.</li>
                </ul>

                <h2>Tips and Tricks</h2>
                <ul>
                    <li><strong>Optimize Workflow:</strong> Use simpler algorithms first to reduce database size before running complex ones like Audio Content.</li>
                    <li><strong>Preserve Original Data:</strong> Enable "Create New Database of Thinned Records" to avoid modifying the original database.</li>
                    <li><strong>Dual Mono Handling:</strong> Use the "Dual Mono Detection" feature to clean up redundant channels.</li>
                </ul>
            </div>

            <!-- Advanced Usage Section -->
            <div id="advanced-usage" class="manual-section">
                <h2>Advanced Usage</h2>
                <p>SMDB Companion has the ability to customize the logic used to select which file should be kept in a set of duplicates. This allow you to help it decide which files you prefer to keep in a set of duplicates.</p>
                <p>This can be configured under <strong>"Preferences -> File Preservation Priority"</strong>.</p>
                <img src="/assets/preservation.jpg">
                <ul>
                    <li><strong>Top Toolbar:</strong> <p>This will help you create new rules to add to the list.</p></li>
                    <li><strong>Rules List:</strong> <p>The list represents the rule order SMDB Companion will use to decide which file to keep.  If all duplicate candidates match the first criteria, it will move to check the next one, so rules at the top of the list are more important than those below.</p></li>
                    <li><strong>Bottom Toolbar:</strong> <p>These buttons allow you to change the order of the list or remove a rule.</p></li>
                </ul>
                
            </div>

            <!-- Troubleshooting and FAQs Section -->
            <div id="tips" class="manual-section">
                <h2>Tips and Tricks</h2>
                <ul>
                    <li><p><strong>Audio Content Algorithm:</strong></p>
                        <p>You will be tempted to use this right away, however as it has to scan each file indiviually, it can be quite slow on large databases.  The recommended workflow is to use the other algorithms first to create a database with less records, and then run this algorithm on the newly created database.</p>
                    </li>
                    <li><p><strong>Create Thinned Database First:</strong></p>
                        <p>It will take some time to get the hang of how SMDB Companion chooses it's duplicates.  Preserve your database by enabled the "Create New Database of Thinned Records" feature.  Once you have a feel for how it works, then you can uncheck this option to manipulate your databases directly.</p>
                    </li>
                    <li><p><strong>Utilize the Duplicates Database:</strong></p> 
                        <p>When you are first working with the program, creating the optional database of duplicates is a faster to way to quickly see which files it is choosing to delete.  Once you have your settings dialed in, this option can be turned off</p>
                    </li>
                    <li><p><strong>Take some time to configure the Perservation Priority:</strong></p>
                        <p> Everyone's library is organized differently, and you will know best which files you want to keep versus which files you want to remove.  In our personal library, we found rules based on filepath to be the most effective for teaching SMDB Companion to preserve the files we wanted.  File Preservation Priority can be found under the 'Preferences" menu at the top.</p>
                    </li>
                    <li><p><strong>Don't Delete Files:</strong></p>
                        <p> While SMDB Companion has the ability to delete files from you disk, the intended work flow is for you to create a new Database without any duplicates and use Soundminer's Mirror function to duplicate it.  This will protect your library from data loss.</p>
                    </li>
                    <li><p><strong>Adding New Media to your Library:</strong></p> 
                        <p>When adding new media to your master library, create a new database just for the media you want to add and then scan and remove duplicates with SMDB Companion.  You can use the compare algorithm to ensure you are not adding duplicates back into your library.</p></li>
                    </ul>
            </div>

            <!-- Contact/Support Information Section -->
            <div id="contact" class="manual-section">
                <h2>Contact/Support Information</h2>
                <p>If you need further assistance, please contact our support team:</p>
                <ul>
                    <li>Email: <a href="mailto:support@smdbc.com">support@smdbc.com</a></li>
                    <li>Website: <a href="https://www.smdbc.com" target="_blank">www.smdbc.com</a></li>
                </ul>
                <?php include 'license-recovery.php'; ?>
            </div>
        </div>
    </div>

    <?php include 'footer.php'; ?>

    <script>
        // Function to show content in the right section based on the sidebar link clicked
        function showContent(sectionId) {
            const sections = document.querySelectorAll('.manual-section');
            sections.forEach(section => {
                section.style.display = 'none'; // Hide all sections
        });

        const activeSection = document.getElementById(sectionId);
        if (activeSection) {
            activeSection.style.display = 'block';
            
            // Ensure the content area scrolls to the top of the section
            const manualContent = document.querySelector('.manual-content');
            manualContent.scrollTop = activeSection.offsetTop - manualContent.offsetTop;
        }
}

        // Initially show the Introduction section
        window.onload = function() {
            const hash = window.location.hash.substring(1); // Remove the '#' from the hash
            if (hash) {
                showContent(hash);
            } else {
                showContent('introduction'); 
            }
        };
    </script>
</body>
</html>
