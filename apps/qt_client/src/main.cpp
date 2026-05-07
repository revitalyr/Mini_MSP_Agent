#include <QApplication>
#include <QStyleFactory>
#include "mainwindow.h"

int main(int argc, char *argv[])
{
    QApplication app(argc, argv);
    
    // Application metadata
    app.setApplicationName("MiniMSPQtClient");
    app.setOrganizationName("MiniMSP");
    app.setApplicationDisplayName("Mini MSP Qt Client");
    
    // Set application style
    app.setStyle(QStyleFactory::create("Fusion"));
    
    // Create and show main window
    MainWindow window;
    window.show();
    
    return app.exec();
}
