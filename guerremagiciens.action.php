<?php
/**
 *------
 * BGA framework: © Gregory Isabelli <gisabelli@boardgamearena.com> & Emmanuel Colin <ecolin@boardgamearena.com>
 * GuerreMagiciens implementation : © <Your name here> <Your email address here>
 *
 * This code has been produced on the BGA studio platform for use on http://boardgamearena.com.
 * See http://en.boardgamearena.com/#!doc/Studio for more information.
 * -----
 * 
 * guerremagiciens.action.php
 *
 * GuerreMagiciens main action entry point
 *
 *
 * In this file, you are describing all the methods that can be called from your
 * user interface logic (javascript).
 *       
 * If you define a method "myAction" here, then you can call it from your javascript code with:
 * this.ajaxcall( "/guerremagiciens/guerremagiciens/myAction.html", ...)
 *
 */
  
  
  class action_guerremagiciens extends APP_GameAction
  { 
    // Constructor: please do not modify
   	public function __default()
  	{
  	    if( self::isArg( 'notifwindow') )
  	    {
            $this->view = "common_notifwindow";
  	        $this->viewArgs['table'] = self::getArg( "table", AT_posint, true );
  	    }
  	    else
  	    {
            $this->view = "guerremagiciens_guerremagiciens";
            self::trace( "Complete reinitialization of board game" );
      }
  	} 
  	
  	// TODO: defines your action entry points there

    public function actionFinishFanaticsDominanceSetup()
    {
        self::setAjaxMode();     

        // Retrieve arguments
        // Note: these arguments correspond to what has been sent through the javascript "ajaxcall" method
        $arg1 = self::getArg( "intSelectedFanaticsToken", AT_int, true );

        // Then, call the appropriate method in your game logic, like "playCard" or "myAction"
        $this->game->gameFinishFanaticsDominanceSetup( $arg1 );

        self::ajaxResponse( );        
    }

    public function actionFinishTownCriersExpense()
    {
        self::setAjaxMode();     

        // Retrieve arguments
        // Note: these arguments correspond to what has been sent through the javascript "ajaxcall" method
        $arg1 = self::getArg( "intGlobalExpenseTotal", AT_posint, true );

        // Then, call the appropriate method in your game logic, like "playCard" or "myAction"
        $this->game->gameFinishTownCriersExpense( $arg1 );

        self::ajaxResponse();        
    }

    public function actionFinishProduceMagicalItem()
    {
        self::setAjaxMode();     

        // Retrieve arguments
        // Note: these arguments correspond to what has been sent through the javascript "ajaxcall" method
        $selected_magical_items_raw = self::getArg( "jsonSelectedItems", AT_numberlist, true );

        if( substr( $selected_magical_items_raw, -1 ) == ';' )
            $selected_magical_items_raw = substr( $selected_magical_items_raw, 0, -1 );

        if( $selected_magical_items_raw == '' )
            $selected_magical_items = array();
        else
            $selected_magical_items = explode( ';', $selected_magical_items_raw );

        // Then, call the appropriate method in your game logic, like "playCard" or "myAction"
        $this->game->gameFinishProduceMagicalItem( $selected_magical_items );

        self::ajaxResponse( );        
    }

    /*
    
    Example:
  	
    public function myAction()
    {
        self::setAjaxMode();     

        // Retrieve arguments
        // Note: these arguments correspond to what has been sent through the javascript "ajaxcall" method
        $arg1 = self::getArg( "myArgument1", AT_posint, true );
        $arg2 = self::getArg( "myArgument2", AT_posint, true );

        // Then, call the appropriate method in your game logic, like "playCard" or "myAction"
        $this->game->myAction( $arg1, $arg2 );

        self::ajaxResponse( );
    }
    
    */

  }
  

