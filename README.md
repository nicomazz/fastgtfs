

<!-- PROJECT SHIELDS -->
![tests](https://github.com/nicomazz/fastgtfs/workflows/Rust/badge.svg)
[![Contributors][contributors-shield]][contributors-url]
[![Forks][forks-shield]][forks-url]
[![Stargazers][stars-shield]][stars-url]
[![Issues][issues-shield]][issues-url]
[![MIT License][license-shield]][license-url]



<!-- PROJECT LOGO -->
<br />
<p align="center">
  <h3 align="center">FastGtfs</h3>

  <a href="https://i.ibb.co/XDqf83x/fastgtfs-cover.png">
   <!-- <img src="images/logo.png" alt="Logo" width="80" height="80">-->
   <img src="https://i.ibb.co/XDqf83x/fastgtfs-cover.png" alt="fastgtfs-cover" border="0">
  </a>


  <p align="center">
    A pure Rust library that provides <a href="https://developers.google.com/transit/gtfs">GTFS</a> parsing, navigation, time table creation and real time network simulation. Already in production with the beta of <a href="https://play.google.com/apps/testing/com.actv.nicomazz.lastjni">this</a> android app. 
    <br />
    <br />
    <br />
    <a href="https://play.google.com/apps/testing/com.actv.nicomazz.lastjni">View Demo</a>
    ·
    <a href="https://github.com/nicomazz/fastgtfs/issues">Report Bug</a>
    ·
    <a href="https://github.com/nicomazz/fastgtfs/issues">Request Feature</a>
  </p>
</p>



<!-- TABLE OF CONTENTS -->
## Table of Contents

* [About the Project](#about-the-project)
* [Usage](#usage)
* [License](#license)
* [Contact](#contact)


<!-- ABOUT THE PROJECT -->
## About The Project


This library is the core of the [Venice Navigation&timetables](https://play.google.com/apps/testing/com.actv.nicomazz.lastjni) app. The app aims to provide services using GTFS data completely **offline**. In this way, tourists without an internet plan do not have to worry about public transport.
This library was all originally written in C++. To learn RUST, I decided to write it all again (so, yes, this is my first rust project). It has been cross-compiled for Android, and provides several features to the app:

- **Navigation** from point A to B, with a slightly modified version of Microsoft's [RAPTOR](https://www.microsoft.com/en-us/research/wp-content/uploads/2012/01/raptor_alenex.pdf) algorithm. This version takes care of real walking path distances and average person preferences when choosing a solution.

- Creation of **timetables** for routes with heterogeneous trips. To do that, I do a topological sort on the different trip paths.

- **Simulation** of the entire network. Given a trip and a time, it is possible to get the bus's exact position by using interpolation. In the android app, it is possible to see a simulation of the entire Venice bus and water-bus network.

- **Merge** of several datasets into a unique data structure.

- **Optimized queries** over merged GTFS datasets. See `gtfs_data.rs` for this.
    
- **Serialization and deserialization** of the raw data structure using Google's [flatbuffers](https://google.github.io/flatbuffers/). In this way, there is a **huge compression**. The Android app uses this feature. Every time the app is started, it reads the binary data (in the flatbuffer format) directly into the final data structure, avoiding the slow txt parsing. The parsing requires more time and generates the serialized binary data.

- **Walk time calculator**. It uses "HERE" APIs to precompute the real walking times between each stop and the 40 nearest ones. This is done with the `walk_distance_calculator` crate. It uses parsed stop positions. This is then used in the navigation algorithm.

There are a few basic tests, where they are really needed. From the Android app, it seems it works well!


<!-- USAGE EXAMPLES -->
## Usage

For the usage, refer to the `tests` folder.


<!-- LICENSE -->
## License

Distributed under the GNU General Public License v3.0 License. See `LICENSE` for more information.



<!-- CONTACT -->
## Contact

Nicolo' Mazzucato - [@nicomazz](https://twitter.com/nicomazz)



<!-- MARKDOWN LINKS & IMAGES -->
<!-- https://www.markdownguide.org/basic-syntax/#reference-style-links -->
[contributors-shield]: https://img.shields.io/github/contributors/nicomazz/fastgtfs.svg?style=flat-square
[contributors-url]: https://github.com/nicomazz/fastgtfs/graphs/contributors
[forks-shield]: https://img.shields.io/github/forks/nicomazz/fastgtfs.svg?style=flat-square
[forks-url]: https://github.com/nicomazz/fastgtfs/network/members
[stars-shield]: https://img.shields.io/github/stars/nicomazz/fastgtfs.svg?style=flat-square
[stars-url]: https://github.com/nicomazz/fastgtfs/stargazers
[issues-shield]: https://img.shields.io/github/issues/nicomazz/fastgtfs.svg?style=flat-square
[issues-url]: https://github.com/nicomazz/fastgtfs/issues
[license-shield]: https://img.shields.io/github/license/nicomazz/fastgtfs.svg?style=flat-square
[license-url]: https://github.com/nicomazz/fastgtfs/blob/master/LICENSE.txt
[linkedin-shield]: https://img.shields.io/badge/-LinkedIn-black.svg?style=flat-square&logo=linkedin&colorB=555
[linkedin-url]: https://linkedin.com/in/nicomazz
[product-screenshot]: images/screenshot.png
